//! 聯想結果的輪詢定時器。發出請求時開始輪詢，結果到了或超時就停，平時不佔 CPU。

use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 輪詢間隔。
const POLL_INTERVAL: f64 = 0.05;

/// 最長輪詢多久；請求本身有超時，這裡只是兜底。
const MAX_WAIT: Duration = Duration::from_secs(12);

pub struct PredictMonitor {
    /// 定時器；沒在等結果時為 `None`。
    timer: Option<Retained<NSTimer>>,

    /// 本輪開始等待的時間。
    since: Option<Instant>,

    mtm: MainThreadMarker,
}

impl PredictMonitor {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self {
            timer: None,
            since: None,
            mtm,
        }
    }

    /// 有請求在飛：開始（或繼續）輪詢。
    pub fn start(&mut self) {
        self.since = Some(Instant::now());
        if self.timer.is_some() {
            return;
        }
        let target = PredictTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                POLL_INTERVAL,
                &target,
                sel!(tick:),
                None,
                true,
            )
        };
        self.timer = Some(timer);
    }

    pub fn stop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
        self.since = None;
    }

    /// 等太久了就放棄。
    pub fn expired(&self) -> bool {
        self.since.is_some_and(|since| since.elapsed() > MAX_WAIT)
    }
}

define_class!(
    // SAFETY: NSObject 沒有子類化要求；沒有實現 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct PredictTicker;

    impl PredictTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.poll_prediction());
        }
    }

    unsafe impl NSObjectProtocol for PredictTicker {}
);

impl PredictTicker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
