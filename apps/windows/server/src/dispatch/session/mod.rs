//! 會話：DLL 每個應用執行緒一條，Server 記宿主應用；焦點在哪個會話，組句就屬於誰。

mod info;

use qingjian_platform::protocol::SessionId;

pub(super) use self::info::SessionInfo;
use super::Router;

impl Router {
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 聚焦會話所在應用的 exe 名；DLL 沒報時為 `None`。
    pub(super) fn focused_app(&self) -> Option<&str> {
        self.focused
            .and_then(|session| self.sessions.get(&session))
            .and_then(|info| info.app.as_deref())
    }

    pub(super) fn ensure_focus(&mut self, session: SessionId) {
        if self.focused != Some(session) {
            self.reset_composition();
            self.focused = Some(session);
            let app = self.focused_app().map(str::to_owned);
            self.engine.set_application(app);
            let private = self.focused_private();
            self.engine.set_private(private);
            self.engine.set_traditional_mode(self.traditional_output());
        }
    }

    /// 聚焦會話該不該出繁體：配置開著就是；關著時看它用的是不是繁體那個語言 profile（Windows「中文(台灣)」下的青簡）。
    pub(super) fn traditional_output(&self) -> bool {
        self.config.traditional
            || self
                .focused
                .and_then(|session| self.sessions.get(&session))
                .is_some_and(|info| info.traditional)
    }

    /// DLL 報來該會話用的語言 profile 是不是繁體的：記下；是當前會話且變了就清掉組句再切 Engine
    /// （組句中切 profile 極少見，候選裡簡繁混著不如從頭來）。
    pub(super) fn set_profile(&mut self, session: SessionId, traditional: bool) {
        let before = self.traditional_output();
        if let Some(info) = self.sessions.get_mut(&session) {
            info.traditional = traditional;
        }
        if self.focused == Some(session) && self.traditional_output() != before {
            self.reset_composition();
            self.engine.set_traditional_mode(self.traditional_output());
        }
    }

    /// 當前聚焦的會話在私密輸入框裡（Engine 不學不記不發雲端）。
    pub fn is_private(&self) -> bool {
        self.engine.is_private()
    }

    fn focused_private(&self) -> bool {
        self.focused
            .and_then(|session| self.sessions.get(&session))
            .is_some_and(|info| info.private)
    }

    /// DLL 報來該會話的輸入框私密與否變了：記下；是當前會話就立刻讓 Engine 進 / 出私密。
    pub(super) fn set_privacy(&mut self, session: SessionId, private: bool) {
        if let Some(info) = self.sessions.get_mut(&session) {
            info.private = private;
        }
        if self.focused == Some(session) {
            self.engine.set_private(private);
        }
    }

    /// 焦點離開：把緩衝區原樣交出並清組句。組句不屬於 `session` 時只清不交，別把 A 應用的拼音落進 B。
    pub(super) fn commit_raw_for(&mut self, session: SessionId) -> Option<String> {
        let text = (self.focused == Some(session) && !self.engine.composition().is_empty())
            .then(|| self.engine.take_raw());
        self.reset_composition();
        text
    }

    /// 清掉組句、展示狀態、在飛的雲聯想與翻譯評審，收起候選視窗。
    pub(super) fn reset_composition(&mut self) {
        self.engine.break_chain();
        self.engine.clear();
        self.cancel_prediction();
        self.stop_rescoring();
        self.composed = None;
        self.translation = None;
        self.pending_selection = None;
        self.sentence = None;
        self.notice = None;
        self.highlight = 0;
        self.navigated = false;
        self.hide_candidate_window();
    }
}
