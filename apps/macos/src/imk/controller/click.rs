//! 滑鼠點候選視窗：點候選與按它的數字鍵一樣上屏，點譯詞與按譯詞鍵一樣上屏它的那條譯詞。
//!
//! 候選視窗不知道是哪個會話，所以在 `activateServer:` 時記下當前控制器，點選時用它的客戶端上屏。

use std::cell::RefCell;

use objc2::Message;
use qingjian_render::HitTarget;

use super::*;

thread_local! {
    /// 最近啟用的會話；`deactivateServer:` 時清掉。
    static ACTIVE: RefCell<Option<Retained<QingjianInputController>>> = const { RefCell::new(None) };
}

/// 候選視窗的點選回撥（host 建視窗時註冊）。
pub fn click(target: HitTarget) {
    let Some(controller) = ACTIVE.with(|active| active.borrow().clone()) else {
        tracing::debug!("没有激活的会话，点击不处理");
        return;
    };
    if catch_panic("click", || controller.click(target)).is_none() {
        recover_from_panic(None);
    }
}

impl QingjianInputController {
    pub(super) fn set_active(&self) {
        ACTIVE.with(|active| *active.borrow_mut() = Some(self.retain()));
    }

    pub(super) fn clear_active(&self) {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active
                .as_deref()
                .is_some_and(|current| std::ptr::eq(current, self))
            {
                *active = None;
            }
        });
    }

    fn click(&self, target: HitTarget) {
        // SAFETY: IMKInputController 的 `client` 返回當前會話的文本客戶端（實現 IMKTextInput 的代理），可能為 nil
        let client: Option<Retained<AnyObject>> = unsafe { msg_send![self, client] };
        let Some(client) = client else {
            return;
        };
        let client = TextClient::new(&client);
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return;
        }
        match target {
            HitTarget::Candidate(offset) => {
                if let Some(index) = host::with(|h| h.session.index_in_view(offset)).flatten() {
                    self.commit_index(index, client);
                }
            }
            HitTarget::Translation { candidate, sense } => {
                let index = host::with(|h| h.session.index_in_view(candidate)).flatten();
                self.commit_translation_at(index, sense, client);
            }
        }
    }
}
