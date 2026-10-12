//! 會話：DLL 每個應用執行緒一條，Server 記宿主應用；焦點在哪個會話，組句就屬於誰。

mod info;

use qingjian_platform::Scheme;
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
            self.apply_session_scheme();
        }
    }

    /// 聚焦會話該不該出繁體：配置開著就是；關著時看它用的是不是繁體的語言 profile（Windows「中文(台灣)」下的青簡 / 青簡注音）。
    /// Windows 版的 Server 配置一律開著（`config::enforce_traditional`），所以實際總是繁體；關著的分支只在 Router 作為庫測試時走到。
    pub(super) fn traditional_output(&self) -> bool {
        self.config.traditional
            || self
                .focused
                .and_then(|session| self.sessions.get(&session))
                .is_some_and(|info| info.traditional)
    }

    /// 聚焦會話的拼音側方案：用的是注音 profile（Windows「中文(台灣)」下的青簡注音）就是大千注音，否則按配置。
    /// 形碼側不受 profile 影響，仍看 `config.wubi`。
    pub(super) fn scheme(&self) -> Scheme {
        let zhuyin = self
            .focused
            .and_then(|session| self.sessions.get(&session))
            .is_some_and(|info| info.zhuyin);
        if zhuyin {
            Scheme::Zhuyin
        } else {
            self.config.scheme
        }
    }

    /// 聚焦會話的方案與 Engine 現在裝的不一致（焦點換到用另一個 profile 的會話）就重新裝配。
    /// 只在注音與否變了時裝：裝配會重讀形碼碼錶，焦點每換一次都裝太浪費。
    fn apply_session_scheme(&mut self) {
        let scheme = self.scheme();
        if self.engine.is_zhuyin_mode() != (scheme == Scheme::Zhuyin) {
            self.reload_code_table(scheme, self.config.wubi);
            self.reconcile_status();
        }
    }

    /// DLL 報來該會話用的語言 profile（繁體與否、注音與否）：記下；是當前會話且有變化就清掉組句再切 Engine
    /// （組句中切 profile 極少見，候選裡簡繁 / 拼音注音混著不如從頭來）。
    /// 不是當前會話、而此刻沒在組句：把焦點給它——profile 是在前臺執行緒上切的，使用者在輸入法列表裡選了
    /// 「青簡注音」，狀態條該立刻顯示注音，不等敲第一個鍵。
    pub(super) fn set_profile(&mut self, session: SessionId, traditional: bool, zhuyin: bool) {
        let before = (self.traditional_output(), self.scheme());
        if let Some(info) = self.sessions.get_mut(&session) {
            info.traditional = traditional;
            info.zhuyin = zhuyin;
        }
        if self.focused != Some(session) {
            if self.engine.composition().is_empty() {
                self.ensure_focus(session);
            }
            return;
        }
        let after = (self.traditional_output(), self.scheme());
        if after == before {
            return;
        }
        self.reset_composition();
        self.engine.set_traditional_mode(after.0);
        self.apply_session_scheme();
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
