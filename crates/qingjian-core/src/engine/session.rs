//! 可掛起的會話輸入狀態；詞庫與使用者學習服務仍由同一個 Engine 持有。

use super::commit::CommitChain;
use super::{Engine, LastCommit, Transition};
use crate::{Composition, InputHistory, Language, Punctuation};
use std::time::Instant;

/// 一個輸入上下文的組句、標點配對和學習鏈；不復制詞庫或落盤服務。
#[derive(Default)]
pub struct EngineSession {
    /// 拼音與游標位置。
    composition: Composition,

    /// 當前中英文模式。
    english_mode: bool,

    /// 中文標點與引號配對。
    punctuation: Punctuation,

    /// 最近上屏記錄，供撤銷錯誤學習。
    recent_commits: Vec<LastCommit>,

    /// 本次上屏記錄的詞轉移。
    recording: Vec<Transition>,

    /// 首次退格前的輸入，供重敲記錄。
    retype_snapshot: Option<String>,

    /// 等待寫入日誌的直通字元。
    passthrough_pending: String,

    /// 本輪候選翻頁次數。
    page_turns: u32,

    /// 本輪組句開始時刻。
    composition_started: Option<Instant>,

    /// 當前輸入應用標識。
    application: Option<String>,

    /// 上次學習鏈斷開後是否已有上屏。
    committed_since_break: bool,

    /// 當前可見候選的譯詞。
    displayed: Vec<(Language, String)>,

    /// 本會話最近的上屏前文。
    history: InputHistory,

    /// 本會話連續上屏的學習鏈。
    chain: CommitChain,
}

impl Engine {
    /// 切換儲存的輸入上下文。呼叫方還應恢復該上下文的私密狀態。
    /// 非同步結果與查詢快取不跨上下文複用，使用者詞頻和詞庫仍是程序內唯一例項。
    pub fn swap_session(&mut self, session: &mut EngineSession) {
        self.cancel_prediction();
        self.set_rescoring_context(None);
        std::mem::swap(&mut self.composition, &mut session.composition);
        std::mem::swap(&mut self.english_mode, &mut session.english_mode);
        std::mem::swap(&mut self.punctuation, &mut session.punctuation);
        std::mem::swap(&mut self.recent_commits, &mut session.recent_commits);
        std::mem::swap(&mut self.recording, &mut session.recording);
        std::mem::swap(&mut self.retype_snapshot, &mut session.retype_snapshot);
        std::mem::swap(
            &mut self.passthrough_pending,
            &mut session.passthrough_pending,
        );
        std::mem::swap(&mut self.page_turns, &mut session.page_turns);
        std::mem::swap(
            &mut self.composition_started,
            &mut session.composition_started,
        );
        std::mem::swap(&mut self.application, &mut session.application);
        std::mem::swap(
            &mut self.committed_since_break,
            &mut session.committed_since_break,
        );
        std::mem::swap(&mut self.displayed, &mut session.displayed);
        std::mem::swap(&mut self.history, &mut session.history);
        std::mem::swap(&mut self.chain, &mut session.chain);
        self.forget_span_cache();
        *self.correction_cache.borrow_mut() = None;
        *self.last_query.borrow_mut() = None;
    }
}

impl EngineSession {
    /// 丟棄本上下文的輸入狀態，不寫日誌、不學習，也不把私密文本帶到下一次普通輸入。
    pub fn discard_input(&mut self) {
        self.composition.clear();
        self.english_mode = false;
        self.punctuation = Punctuation::default();
        self.recent_commits.clear();
        self.recording.clear();
        self.retype_snapshot = None;
        self.passthrough_pending.clear();
        self.page_turns = 0;
        self.composition_started = None;
        self.committed_since_break = false;
        self.displayed.clear();
        self.history.clear();
        self.chain = CommitChain::default();
    }
}

impl Engine {
    /// 在隱私邊界丟棄當前裝入的輸入狀態。與 [`Self::set_private`] 分開，避免平臺殼
    /// 在組句第一幀後報告隱私狀態時意外清掉新輸入。
    pub fn discard_input(&mut self) {
        self.cancel_prediction();
        self.composition.clear();
        self.english_mode = false;
        self.punctuation = Punctuation::default();
        self.recent_commits.clear();
        self.recording.clear();
        self.retype_snapshot = None;
        self.passthrough_pending.clear();
        self.page_turns = 0;
        self.composition_started = None;
        self.committed_since_break = false;
        self.displayed.clear();
        self.history.clear();
        self.chain = CommitChain::default();
        self.rescoring_before = None;
        self.last_prediction_scope.clear();
        self.last_question_guess.clear();
        *self.neural_cache.borrow_mut() = super::rescoring::NeuralCache::default();
        *self.correction_cache.borrow_mut() = None;
        *self.last_query.borrow_mut() = None;
        self.forget_span_cache();
        self.traditional_map.borrow_mut().clear();
    }
}
