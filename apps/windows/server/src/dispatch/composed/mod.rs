//! 組句的展示狀態：緩衝變化時重查候選並重建 [`Composed`]，雲端詞非同步併入，高亮 / 翻頁，按狀態生成給 DLL 的幀。

mod state;

use qingjian_core::{Candidate, CandidateLayout, CandidateList, CloudWord, Query};
use qingjian_platform::protocol::{Frame, PROTOCOL_VERSION, PreeditKind, PreeditSegment};

pub(super) use self::state::{Composed, TypedKeys};
use super::Router;

impl Router {
    /// 緩衝變化後：按 Engine 狀態重建 [`Composed`]，發一次雲聯想請求，歸零高亮與整句補全。
    pub(super) fn recompose(&mut self) {
        self.highlight = 0;
        self.navigated = false;
        self.sentence = None;
        if self.engine.composition().is_empty() {
            self.composed = None;
            self.cancel_prediction();
            self.stop_rescoring();
            return;
        }
        self.attach_loaded_model();
        let built = self.engine.query().ok().map(|query| {
            let (preedit, cursor, typed_keys) = marked_parts(&query);
            (query.candidates.items.clone(), preedit, cursor, typed_keys)
        });
        self.composed = Some(match built {
            Some((items, preedit, cursor, typed_keys)) => {
                let layout =
                    CandidateLayout::new(items, self.config.page_size, self.config.cloud_slots);
                if self.engine.prediction_enabled() {
                    self.engine.request_prediction(None, layout.local());
                }
                Composed::Candidates {
                    preedit,
                    cursor,
                    typed_keys,
                    layout,
                }
            }
            None => {
                self.cancel_prediction();
                let composition = self.engine.composition();
                let text = composition.text().to_owned();
                let cursor = text[..composition.cursor()].chars().count();
                Composed::Raw { text, cursor }
            }
        });
        self.schedule_rescoring();
    }

    /// 拉一次雲聯想結果：雲端詞並進候選佈局，整句補全記下；翻譯評審時結果是譯文。
    pub(super) fn poll_prediction(&mut self) {
        if !self.engine.prediction_enabled() {
            return;
        }
        let Some(prediction) = self.engine.poll_prediction() else {
            return;
        };
        if self.translation.is_some() {
            match prediction.sentence {
                Some(text) => {
                    if let Some(translation) = self.translation.as_mut() {
                        translation.result = Some(text);
                    }
                }
                None => {
                    tracing::info!("翻譯選中文字：雲端沒有給出譯文");
                    self.translation = None;
                }
            }
            return;
        }
        if let Some(Composed::Candidates { layout, .. }) = self.composed.as_mut() {
            let words: Vec<Candidate> = prediction
                .words
                .into_iter()
                .map(CloudWord::into_candidate)
                .collect();
            layout.set_cloud(words);
            self.sentence = prediction.sentence;
        }
    }

    pub(super) fn cancel_prediction(&mut self) {
        if self.engine.prediction_enabled() {
            self.engine.cancel_prediction();
        }
    }

    /// 高亮移動 `delta`，夾在 `[0, 末尾]`，到頁邊自然換頁。
    pub(super) fn move_highlight(&mut self, delta: isize) {
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let next = (self.highlight as isize + delta).clamp(0, count as isize - 1) as usize;
        self.navigated |= next != self.highlight;
        self.highlight = next;
    }

    /// 整頁翻 `step`，高亮落到目標頁第一個候選。
    pub(super) fn page(&mut self, step: isize) {
        let count = self.candidate_count();
        if count == 0 {
            self.highlight = 0;
            return;
        }
        let page_size = self.config.page_size;
        let page_count = count.div_ceil(page_size);
        let current = (self.highlight / page_size) as isize;
        let target = (current + step).clamp(0, page_count as isize - 1) as usize;
        if target != current as usize {
            self.navigated = true;
            self.engine.note_page_turn();
        }
        self.highlight = (target * page_size).min(count - 1);
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
    /// 焦點會話的 DLL 比 Server 老時按老協議降級（見 [`Self::downgrade_for_old_dll`]）。
    pub(super) fn current_frame(&self) -> Frame {
        let mut frame = self.raw_frame();
        self.show_typed_keys(&mut frame);
        self.downgrade_for_old_dll(&mut frame);
        frame
    }

    /// 給 DLL 的幀只管應用輸入框：雙拼「輸入框顯示原始按鍵」開著時換成敲的鍵，拼音行由 Server 自繪照舊全拼。
    fn show_typed_keys(&self, frame: &mut Frame) {
        if self.translation.is_some() {
            return;
        }
        let Some(Composed::Candidates {
            typed_keys: Some(keys),
            ..
        }) = &self.composed
        else {
            return;
        };
        frame.preedit = vec![PreeditSegment {
            text: keys.text.clone(),
            kind: PreeditKind::Typed,
        }];
        frame.cursor = keys.cursor;
    }

    /// 自繪候選窗用的幀：不做老 DLL 降級，碼段照常畫。
    pub(super) fn self_drawn_frame(&self) -> Frame {
        self.raw_frame()
    }

    /// 協議比 Server 老的 DLL 不認識 `AuxCode` 段，收到會整條訊息解析失敗；給它的碼段降級成普通拼音段。
    fn downgrade_for_old_dll(&self, frame: &mut Frame) {
        if self.focused_dll_protocol() >= PROTOCOL_VERSION {
            return;
        }
        for segment in &mut frame.preedit {
            if segment.kind == PreeditKind::AuxCode {
                segment.kind = PreeditKind::Typed;
            }
        }
    }

    /// 焦點會話的 DLL 協議版本；沒有焦點會話時按最新（不必降級）。
    pub(super) fn focused_dll_protocol(&self) -> u32 {
        self.focused
            .and_then(|session| self.sessions.get(&session))
            .map_or(PROTOCOL_VERSION, |info| info.protocol)
    }

    fn raw_frame(&self) -> Frame {
        if let Some(translation) = &self.translation {
            return self.translation_frame(translation);
        }
        match &self.composed {
            None => Frame::default(),
            Some(Composed::Raw { text, cursor }) => Frame {
                preedit: vec![PreeditSegment {
                    text: text.clone(),
                    kind: PreeditKind::Typed,
                }],
                preedit_mode: self.config.preedit,
                cursor: *cursor,
                candidates: CandidateList { items: Vec::new() },
                highlight: usize::MAX,
                page: 0,
                page_count: 1,
                layout: self.config.layout,
                appearance: self.config.appearance,
                aux_code_show: self.config.aux_code_show,
                sentence: None,
                notice: self.notice.clone(),
            },
            Some(Composed::Candidates {
                preedit,
                cursor,
                layout,
                ..
            }) => {
                let page_size = self.config.page_size;
                let highlight = self.highlight.min(layout.len().saturating_sub(1));
                let page = highlight / page_size;
                let items: Vec<Candidate> = layout
                    .page(page)
                    .into_iter()
                    .filter_map(|cell| cell.candidate().cloned())
                    .collect();
                let mut candidates = CandidateList { items };
                self.engine.annotate(&mut candidates);
                Frame {
                    preedit: preedit.clone(),
                    preedit_mode: self.config.preedit,
                    cursor: *cursor,
                    candidates,
                    highlight: highlight - page * page_size,
                    page,
                    page_count: layout.pages().max(1),
                    layout: self.config.layout,
                    appearance: self.config.appearance,
                    aux_code_show: self.config.aux_code_show,
                    sentence: self.sentence.clone(),
                    notice: self.notice.clone(),
                }
            }
        }
    }
}

/// 一次查詢的拼音行分段與游標，以及輸入框另顯示原始按鍵時的那一串。
/// 游標用 Core 的對映：自動補的 `'` 會讓顯示串比敲的長。
pub(super) fn marked_parts(query: &Query) -> (Vec<PreeditSegment>, usize, Option<TypedKeys>) {
    let preedit = query.marked_segments().iter().map(Into::into).collect();
    let typed_keys = query.shuangpin_raw_preedit.then(|| TypedKeys {
        text: query.marked_text(),
        cursor: query.marked_cursor(),
    });
    (preedit, query.segments_cursor(), typed_keys)
}
