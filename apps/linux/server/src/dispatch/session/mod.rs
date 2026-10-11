//! 上下文掛起與恢復；其他會話的通知不會清空當前輸入。
mod info;
pub(super) use self::info::SessionInfo;
use super::Router;
use qingjian_platform::protocol::SessionId;

impl Router {
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
    pub fn is_private(&self) -> bool {
        self.engine.is_private()
    }
    pub(super) fn focused_app(&self) -> Option<&str> {
        self.focused
            .and_then(|id| self.sessions.get(&id))
            .and_then(|s| s.app.as_deref())
    }
    pub(super) fn ensure_focus(&mut self, session: SessionId) {
        if self.focused == Some(session) {
            return;
        }
        self.stop_rescoring();
        if let Some(previous) = self.focused.and_then(|id| self.sessions.get_mut(&id)) {
            self.engine.note_displayed(std::iter::empty());
            self.engine.swap_session(&mut previous.engine);
            previous.composed = self.composed.take();
            previous.highlight = self.highlight;
            previous.navigated = self.navigated;
            previous.display_frame = None;
            previous.last_frame = None;
            previous.shift_pending = false;
        }
        let Some(next) = self.sessions.get_mut(&session) else {
            return;
        };
        self.engine.swap_session(&mut next.engine);
        self.composed = next.composed.take();
        self.highlight = next.highlight;
        self.navigated = next.navigated;
        self.engine.set_application(next.app.clone());
        self.engine.set_private(next.private);
        self.notice = None;
        self.sentence = None;
        self.focused = Some(session);
    }
    pub(super) fn set_privacy(&mut self, session: SessionId, private: bool) {
        let Some(info) = self.sessions.get_mut(&session) else {
            return;
        };
        if info.private == private {
            return;
        }
        info.private = private;
        info.display_frame = None;
        info.last_frame = None;
        if let Some(identity) = &mut info.display_identity {
            self.display_revision += 1;
            identity.revision = self.display_revision;
        }
        info.composed = None;
        info.highlight = 0;
        info.navigated = false;
        // 真正的隱私能力變化是輸入邊界。僅在切換上下文時恢復 private 不走這裡，
        // 因此普通與私密會話來回切換不會丟掉各自尚未上屏的組句。
        if self.focused == Some(session) {
            self.stop_rescoring();
            self.engine.discard_input();
            self.engine.set_private(private);
            self.composed = None;
            self.sentence = None;
            self.notice = None;
            self.highlight = 0;
            self.navigated = false;
        } else {
            info.engine.discard_input();
        }
    }
    pub(super) fn reset_composition(&mut self) {
        self.stop_rescoring();
        self.engine.break_chain();
        self.engine.clear();
        self.composed = None;
        self.sentence = None;
        self.notice = None;
        self.highlight = 0;
        self.navigated = false;
    }
    pub(super) fn close_session(&mut self, session: SessionId) {
        if !self.sessions.contains_key(&session) {
            return;
        }
        let previous = self.focused.filter(|id| *id != session);
        // 掛起會話也必須按自己的 private 狀態結束：普通透傳寫日誌，私密透傳靜默丟棄。
        self.ensure_focus(session);
        self.reset_composition();
        if let Some(info) = self.sessions.get_mut(&session) {
            self.engine.swap_session(&mut info.engine);
        }
        self.focused = None;
        self.sessions.remove(&session);
        if let Some(previous) = previous {
            self.ensure_focus(previous);
        }
        self.flush_learning();
    }
}
