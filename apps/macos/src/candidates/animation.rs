//! 候選窗動畫的定時器：渲染器說還有動畫在播（`Rendered::next_frame`）就按它給的間隔起，每跳要一幀新點陣圖；播完或視窗收起就停，平時不佔 CPU。

use std::time::Duration;

use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

use super::view::CandidateView;

#[derive(Default)]
pub struct AnimationTimer {
    /// 在跳的定時器與它的間隔；沒有動畫時為 `None`。
    timer: Option<(Retained<NSTimer>, Duration)>,
}

impl AnimationTimer {
    /// 按 `interval` 跳（渲染器給的：過渡 16 ms、迴圈動畫 33 ms）；已按同樣間隔在跳就不動，間隔變了重起。
    /// 每跳調 `view` 的 [`CandidateView::animation_frame`]。
    pub fn start(&mut self, mtm: MainThreadMarker, view: &CandidateView, interval: Duration) {
        if self
            .timer
            .as_ref()
            .is_some_and(|(_, running)| *running == interval)
        {
            return;
        }
        self.stop();
        let target = AnimationTicker::new(mtm, view);
        // SAFETY: 選擇子 `tick:` 由 AnimationTicker 實現，簽名是定時器回撥要的 `(NSTimer)`。
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                interval.as_secs_f64(),
                &target,
                sel!(tick:),
                None,
                true,
            )
        };
        self.timer = Some((timer, interval));
    }

    pub fn stop(&mut self) {
        if let Some((timer, _)) = self.timer.take() {
            timer.invalidate();
        }
    }
}

/// 定時器的目標：弱引用檢視，檢視沒了就什麼也不做。
pub struct TickerIvars {
    view: Weak<CandidateView>,
}

define_class!(
    // SAFETY: NSObject 沒有子類化要求；沒有實現 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = TickerIvars]
    struct AnimationTicker;

    impl AnimationTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            if let Some(view) = self.ivars().view.load() {
                view.animation_frame();
            }
        }
    }

    unsafe impl NSObjectProtocol for AnimationTicker {}
);

impl AnimationTicker {
    fn new(mtm: MainThreadMarker, view: &CandidateView) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(TickerIvars {
            view: Weak::from(view),
        });
        // SAFETY: NSObject 的 init
        unsafe { msg_send![super(this), init] }
    }
}
