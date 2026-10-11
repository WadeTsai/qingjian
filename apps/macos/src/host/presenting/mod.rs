//! 呈現：刪候選、按應用關英文候選、翻譯選區的起止、提示氣泡、會話重置與候選視窗繪製。

mod notice;
mod translation_job;

pub(super) use notice::Notice;
pub use translation_job::TranslationJob;

use super::cloud::cloud_candidate;
use super::*;

impl Host {
    /// 刪掉當前頁第 `offset` 格的候選：使用者詞整個刪、詞庫詞清學習。返回給使用者看的一句話；那格沒有候選返回 `None`。
    pub fn forget_candidate(&mut self, offset: usize) -> Option<String> {
        let index = self.session.index_on_page(offset)?;
        let candidate = self.session.candidate(index)?;
        let forgotten = self.engine.forget(&candidate);
        let text = &candidate.text;
        Some(if forgotten.user_word {
            format!("已删除用户词「{text}」")
        } else if forgotten.learning {
            format!("已忘掉对「{text}」的学习记录")
        } else {
            format!("「{text}」是词库里的词，也没有学习记录，没什么可删")
        })
    }

    /// 這個應用裡英文模式給不給候選：全域性開關開著，且應用在 `[apps] english_candidates_on` 白名單裡。
    pub fn english_candidates_in(&self, bundle: Option<&str>) -> bool {
        self.english_candidates && self.apps.english_candidates_on(bundle)
    }

    /// 開始一次翻譯：記下選區，視窗先顯示「翻譯中…」。呼叫方已發出請求。
    pub fn begin_translation(&mut self, range: objc2_foundation::NSRange) {
        self.translation = Some(TranslationJob {
            range,
            result: None,
        });
        self.reset_session(None, vec![cloud_candidate("翻译中…".to_owned())]);
        self.await_prediction();
        self.render();
    }

    /// 在候選窗口裡顯示一行提示，幾秒後自動收起（敲鍵也收）。
    pub fn show_notice(&mut self, text: &str, anchor: NSRect) {
        self.anchor = anchor;
        self.reset_session(None, vec![cloud_candidate(text.to_owned())]);
        self.render();
        let mtm = MainThreadMarker::new().expect("Host 只在主线程用");
        self.notice = Some(Notice::schedule(mtm));
    }

    /// 收起提示；沒在顯示就什麼都不做。
    pub fn clear_notice(&mut self) {
        if self.notice.take().is_some() && self.translation.is_none() {
            self.reset_session(None, Vec::new());
            self.window.hide();
        }
    }

    /// 翻譯結束（接受、放棄或失敗）：收窗、停輪詢。
    pub fn end_translation(&mut self) {
        if self.translation.take().is_some() {
            self.cancel_prediction();
            self.reset_session(None, Vec::new());
            self.window.hide();
        }
    }

    /// 橫排矩陣這套按鍵是否生效：開關開著（`[general] horizontal_grid`）、排布是橫排，而且在用系統繪製。
    /// 青簡渲染器還沒有矩陣視窗，用它時不展開，橫排照單行走（見 `docs/notes/crate-notes.md`）。
    pub fn grid_keys(&self) -> bool {
        self.horizontal_grid && self.layout == LayoutMode::Horizontal && !self.window.uses_bitmap()
    }

    /// 新一輪候選：每頁格數取配置與視窗能畫的行數中較小者，雲端槽位數取配置。
    pub fn reset_session(&mut self, preedit: Option<Preedit>, candidates: Vec<Candidate>) {
        self.status = None;
        let page_size = self.page_size.min(self.window.max_rows()).max(1);
        self.session
            .reset(preedit, candidates, page_size, self.cloud_slots);
    }

    /// 按會話狀態畫候選視窗。候選為空且沒有 preedit 時收窗。
    pub fn render(&mut self) {
        let size = self.session.layout.page_size();
        let page = self.session.page;
        // 橫排展開成矩陣時畫視口裡的幾行，序號只標在高亮所在那一行（數字鍵選的就是它）；單行時只畫當前頁
        let grid = self.session.grid_cells();
        let first = self.session.grid.map_or(page, |grid| grid.top()) * size;
        let (cells, columns) = match &grid {
            Some((cells, columns)) => (cells.clone(), *columns),
            None => (self.session.page_cells(), 0),
        };
        let rows: Vec<Row> = cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let offset = i % size;
                let labelled = columns == 0 || (first + i) / size == page;
                let index = if labelled {
                    (offset + 1).to_string()
                } else {
                    String::new()
                };
                let Some(candidate) = cell.candidate() else {
                    return Row {
                        // 矩陣裡的空位什麼都不畫；單行裡的空位留著序號
                        index: if columns == 0 { index } else { String::new() },
                        text: String::new(),
                        annotation: Vec::new(),
                        cloud: false,
                    };
                };
                let mut row = Row::from_candidate(offset, candidate);
                row.index = index;
                row.cloud = candidate.kind == CandidateKind::Cloud;
                row
            })
            .collect();
        // 頁上的譯詞告訴 Engine：使用者上屏那一刻它們在螢幕上，算「見過」（詞彙記錄）；視窗收起時傳空
        self.engine
            .note_displayed(cells.iter().copied().filter_map(Cell::candidate));
        // 配置成只在行內顯示時，視窗頂部不畫拼音行
        let preedit = self
            .preedit_mode
            .in_window()
            .then(|| self.session.preedit.clone())
            .flatten();
        if rows.is_empty() && self.session.preedit.is_none() {
            self.window.hide();
            return;
        }
        let pages = self.session.pages();
        let footer = (pages > 1).then(|| format!("{}/{pages}", page + 1));
        let frame = Frame {
            preedit,
            rows,
            highlighted: self.session.highlighted.saturating_sub(first),
            columns,
            column_ems: if columns > 0 {
                qingjian_core::Grid::column_ems(&self.session.layout)
            } else {
                Vec::new()
            },
            footer,
            sentence: self.sentence.clone(),
            status: self.status.clone(),
            mode: self.input_mode(),
        };
        self.window.show(frame, self.anchor);
    }

    /// 給候選視窗主題顯示的輸入狀態：中英看 Caps Lock（與選單欄一致），其餘看配置；英文模式標點是半形。
    fn input_mode(&self) -> qingjian_render::Mode {
        let general = &self.settings.config().general;
        let english = crate::imk::modifiers::caps_lock_on();
        qingjian_render::Mode {
            english,
            traditional: general.traditional,
            full_width: !english && general.full_width_punctuation,
            scheme: qingjian_platform::scheme_name(general.scheme(), general.wubi()),
        }
    }
}
