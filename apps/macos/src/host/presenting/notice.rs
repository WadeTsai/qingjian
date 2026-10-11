//! 候選窗口裡一閃而過的提示（「沒有選中的文字」這類），幾秒後自動收起，敲任何鍵也收起。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 提示顯示多久。
const NOTICE_SECONDS: f64 = 2.5;

/// 一條正在顯示的提示：到點自動收。
pub struct Notice {
    /// 收起用的定時器。
    timer: Retained<NSTimer>,
}

impl Notice {
    /// 起一個一次性定時器，到點讓 Host 收提示。
    pub fn schedule(mtm: MainThreadMarker) -> Self {
        let target = NoticeTicker::new(mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                NOTICE_SECONDS,
                &target,
                sel!(tick:),
                None,
                false,
            )
        };
        Self { timer }
    }
}

impl Drop for Notice {
    fn drop(&mut self) {
        self.timer.invalidate();
    }
}

define_class!(
    // SAFETY: NSObject 沒有子類化要求；沒有實現 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct NoticeTicker;

    impl NoticeTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.clear_notice());
        }
    }

    unsafe impl NSObjectProtocol for NoticeTicker {}
);

impl NoticeTicker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
