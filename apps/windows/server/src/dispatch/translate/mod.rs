//! 「翻譯選中文字」：快捷鍵 → 請 DLL 讀選區 → 雲端翻譯 → 候選窗顯示譯文，回車 / 空格替換、Esc 保留。
//! 與 macOS 殼對齊；引擎呼叫在 Core。替換選區靠回給 DLL 的 `commit`：無組句時 `InsertTextAtSelection` 正好替換選區。
//! 進行態在 [`Translation`]。

mod job;

use qingjian_core::{Candidate, CandidateKind, CandidateList};
use qingjian_platform::protocol::{
    Frame, KeyEvent, KeyModifiers, KeyOutcome, ScreenRect, ServerMessage, SessionId,
};

pub(super) use self::job::Translation;
use super::Router;
use super::key::{ESCAPE, RETURN};

impl Router {
    /// 修飾鍵比物理組合（去掉 Caps / 中英模式兩個狀態位）。
    pub(super) fn matches_translate_combo(&self, event: &KeyEvent) -> bool {
        let combo = self.config.translate_selection;
        event.character == Some(combo.key)
            && event.modifiers.chord() == KeyModifiers::from(combo.modifiers)
    }

    /// DLL 回來的選區：非空且雲服務開著就進入評審；否則回空幀讓 DLL 清掉本地翻譯態。
    /// 回給 DLL 的幀恆空（候選窗在 Server 自繪）。
    pub(super) fn handle_selection(
        &mut self,
        session: SessionId,
        request: u64,
        text: String,
        rect: ScreenRect,
    ) -> ServerMessage {
        let empty = ServerMessage::KeyResult {
            session,
            outcome: KeyOutcome::Consumed,
            commit: None,
            frame: Frame::default(),
        };
        if self.focused != Some(session) || self.pending_selection != Some(request) {
            return empty;
        }
        self.pending_selection = None;
        let text = text.trim();
        if text.is_empty() || !self.engine.prediction_enabled() {
            tracing::info!("翻譯選中文字：沒有可讀的選區（或雲服務已關）");
            return empty;
        }
        self.last_rect = Some(rect);
        self.translation = Some(Translation { result: None });
        self.engine.request_translation(text);
        tracing::debug!(chars = text.chars().count(), "翻譯選中文字：已發翻譯請求");
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
        empty
    }

    /// 評審態收到按鍵：任何鍵都結束評審。回車 / 空格 / 1 接受（譯文沒到就只吃鍵），Esc 放棄，其餘放棄並交回應用。
    pub(super) fn handle_translation_review(
        &mut self,
        session: SessionId,
        event: &KeyEvent,
    ) -> ServerMessage {
        let accept =
            event.virtual_key == RETURN || matches!(event.character, Some(' ') | Some('1'));
        let escape = event.virtual_key == ESCAPE;
        let result = self
            .translation
            .as_ref()
            .and_then(|translation| translation.result.clone());
        self.end_translation();
        let (commit, outcome) = if accept {
            (result, KeyOutcome::Consumed)
        } else if escape {
            (None, KeyOutcome::Consumed)
        } else {
            (None, KeyOutcome::Passthrough)
        };
        ServerMessage::KeyResult {
            session,
            outcome,
            commit,
            frame: Frame::default(),
        }
    }

    /// 結束評審：丟進行態、取消在飛的請求、收起候選窗。沒在評審時是空操作。
    pub(super) fn end_translation(&mut self) {
        if self.translation.take().is_some() {
            self.cancel_prediction();
            self.hide_candidate_window();
        }
    }

    /// 評審幀：單條候選（譯文或「翻譯中…」），無 preedit、無分頁。
    pub(super) fn translation_frame(&self, translation: &Translation) -> Frame {
        let text = translation
            .result
            .clone()
            .unwrap_or_else(|| "翻譯中…".to_owned());
        Frame {
            preedit: Vec::new(),
            preedit_mode: self.config.preedit,
            cursor: 0,
            candidates: CandidateList {
                items: vec![translate_candidate(text)],
            },
            highlight: 0,
            page: 0,
            page_count: 1,
            layout: self.config.layout,
            appearance: self.config.appearance,
            aux_code_show: self.config.aux_code_show,
            sentence: None,
            notice: None,
        }
    }
}

/// 譯文包成一條雲端樣式的候選（帶雲朵標記）。
fn translate_candidate(text: String) -> Candidate {
    Candidate {
        text,
        kind: CandidateKind::Cloud,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    }
}
