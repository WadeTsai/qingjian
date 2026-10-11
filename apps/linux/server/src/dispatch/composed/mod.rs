//! 組句的展示狀態：緩衝變化時重查候選並重建 [`Composed`]，雲端詞非同步併入，高亮 / 翻頁，按狀態生成給 DLL 的幀。

mod state;

use qingjian_core::{Candidate, CandidateKind, CandidateLayout, CandidateList};
use qingjian_platform::protocol::{Frame, PreeditKind, PreeditSegment};

pub(super) use self::state::Composed;
use super::Router;

impl Router {
    /// 緩衝變化後：按 Engine 狀態重建 [`Composed`]，發一次雲聯想請求，歸零高亮與整句補全。
    pub(super) fn recompose(&mut self) {
        self.highlight = 0;
        self.navigated = false;
        self.sentence = None;
        if self.engine.composition().is_empty() {
            self.composed = None;
            self.stop_rescoring();
            return;
        }
        self.attach_loaded_model();
        let built = self.engine.query().ok().map(|query| {
            let items = query.candidates.items.clone();
            let preedit: Vec<PreeditSegment> =
                query.marked_segments().iter().map(Into::into).collect();
            // 游標用 Core 的對映：自動補的 `'` 會讓顯示串比敲的長。
            (items, preedit, query.marked_cursor())
        });
        self.composed = Some(match built {
            Some((items, preedit, cursor)) => {
                let layout =
                    CandidateLayout::new(items, self.config.page_size, self.config.cloud_slots);
                Composed::Candidates {
                    preedit,
                    cursor,
                    layout,
                }
            }
            None => {
                let composition = self.engine.composition();
                let text = composition.text().to_owned();
                let cursor = text[..composition.cursor()].chars().count();
                Composed::Raw { text, cursor }
            }
        });
        self.highlight = (0..self.candidate_count())
            .find(|&index| self.layout_candidate(index).is_some())
            .unwrap_or(0);
        self.schedule_rescoring();
    }

    /// 高亮移動 `delta`，夾在 `[0, 末尾]`，到頁邊自然換頁。
    pub(super) fn move_highlight(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let mut next = self.highlight as isize + delta.signum();
        while (0..count as isize).contains(&next) {
            if self.layout_candidate(next as usize).is_some() {
                self.navigated = true;
                self.highlight = next as usize;
                break;
            }
            next += delta.signum();
        }
    }

    /// 整頁翻 `step`，高亮落到目標頁第一個候選。
    pub(super) fn page(&mut self, step: isize) {
        if step == 0 {
            return;
        }
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let page_size = self.config.page_size;
        let page_count = count.div_ceil(page_size);
        let current = (self.highlight / page_size) as isize;
        let mut target = (current + step).clamp(0, page_count as isize - 1);
        while (0..page_count as isize).contains(&target) {
            let start = target as usize * page_size;
            if let Some(index) = (start..(start + page_size).min(count))
                .find(|&index| self.layout_candidate(index).is_some())
            {
                if target != current {
                    self.navigated = true;
                    self.engine.note_page_turn();
                }
                self.highlight = index;
                break;
            }
            target += step.signum();
        }
    }

    pub(super) fn candidate_count(&self) -> usize {
        match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.len(),
            _ => 0,
        }
    }

    /// 候選佈局裡第 `index` 個（跨頁下標）。
    pub(super) fn layout_candidate(&self, index: usize) -> Option<Candidate> {
        match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.candidate(index).cloned(),
            _ => None,
        }
    }

    pub(super) fn commit_index(&mut self, index: usize) -> Option<String> {
        let candidate = self.layout_candidate(index)?;
        Some(self.engine.commit(&candidate))
    }

    /// 按當前狀態生成一幀：翻譯評審優先；沒在組句給空幀；否則給高亮所在的那一頁。
    pub(super) fn current_frame(&self) -> Frame {
        match &self.composed {
            None => Frame::default(),
            Some(Composed::Raw { text, cursor }) => Frame {
                preedit: vec![PreeditSegment {
                    text: text.clone(),
                    kind: PreeditKind::Typed,
                }],
                cursor: *cursor,
                preedit_mode: self.config.preedit,
                candidates: CandidateList { items: Vec::new() },
                highlight: usize::MAX,
                page: 0,
                page_count: 1,
                layout: self.config.layout,
                appearance: self.config.appearance,
                aux_code_show: false,
                sentence: None,
                notice: self.notice.clone(),
            },
            Some(Composed::Candidates {
                preedit,
                cursor,
                layout,
            }) => {
                let page_size = self.config.page_size;
                let highlight = self.highlight.min(layout.len().saturating_sub(1));
                let page = highlight / page_size;
                let items: Vec<Candidate> = layout
                    .page(page)
                    .into_iter()
                    // 空文本只用於顯示佔位，不傳給 Engine 上屏；保持數字標籤與固定位置一致。
                    .map(|cell| {
                        cell.candidate().cloned().unwrap_or_else(|| Candidate {
                            text: String::new(),
                            kind: CandidateKind::Chinese,
                            syllables: Vec::new(),
                            reading: None,
                            translation: None,
                            aux_code: None,
                        })
                    })
                    .collect();
                let mut candidates = CandidateList { items };
                self.engine.annotate(&mut candidates);
                Frame {
                    preedit: preedit.clone(),
                    cursor: *cursor,
                    preedit_mode: self.config.preedit,
                    candidates,
                    highlight: highlight - page * page_size,
                    page,
                    page_count: layout.pages().max(1),
                    layout: self.config.layout,
                    appearance: self.config.appearance,
                    aux_code_show: false,
                    sentence: self.sentence.clone(),
                    notice: self.notice.clone(),
                }
            }
        }
    }
}
