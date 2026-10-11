//! 本地整句模型的三個定時器：等後臺載入接上、停鍵後的防抖（到點才把整句路徑送去後臺打分）與結果輪詢（到了就重畫，平時不佔 CPU）。

use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 停鍵多久才請求重排：比一般的擊鍵間隔短，連著敲時不請求。
const DEBOUNCE: f64 = 0.08;

/// 輪詢間隔：模型一次二三十毫秒。
const POLL_INTERVAL: f64 = 0.02;

/// 最長等多久；後臺執行緒卡住時兜底。
const MAX_WAIT: Duration = Duration::from_secs(2);

/// 等模型載入的間隔：載入要幾百毫秒到幾秒，接上晚幾十毫秒無妨。
const LOAD_INTERVAL: f64 = 0.1;

pub struct RescoreMonitor {
    /// 等模型載入的定時器；沒在載入為 `None`。
    loading: Option<Retained<NSTimer>>,

    /// 防抖定時器（一次性）；沒在等為 `None`。
    debounce: Option<Retained<NSTimer>>,

    /// 輪詢定時器；沒在等結果為 `None`。
    poll: Option<Retained<NSTimer>>,

    /// 本輪開始等結果的時間。
    since: Option<Instant>,

    mtm: MainThreadMarker,
}

impl RescoreMonitor {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self {
            loading: None,
            debounce: None,
            poll: None,
            since: None,
            mtm,
        }
    }

    /// 模型在後臺載入：定時看一眼接上沒有。按鍵路徑也會看，但使用者停鍵後沒人再看，接上的那一刻就沒人知道。
    pub fn watch_loading(&mut self) {
        if self.loading.is_some() {
            return;
        }
        let target = RescoreTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                LOAD_INTERVAL,
                &target,
                sel!(attach:),
                None,
                true,
            )
        };
        self.loading = Some(timer);
    }

    /// 載入有結果了（接上、失敗或卸掉）：不再看。
    pub fn stop_watching(&mut self) {
        if let Some(timer) = self.loading.take() {
            timer.invalidate();
        }
    }

    /// 又敲了一鍵：重新計時。
    pub fn schedule(&mut self) {
        if let Some(timer) = self.debounce.take() {
            timer.invalidate();
        }
        let target = RescoreTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                DEBOUNCE,
                &target,
                sel!(fire:),
                None,
                false,
            )
        };
        self.debounce = Some(timer);
    }

    /// 請求已發出：開始輪詢結果。
    pub fn start_polling(&mut self) {
        self.since = Some(Instant::now());
        if self.poll.is_some() {
            return;
        }
        let target = RescoreTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                POLL_INTERVAL,
                &target,
                sel!(poll:),
                None,
                true,
            )
        };
        self.poll = Some(timer);
    }

    /// 停下兩個定時器。
    pub fn stop(&mut self) {
        if let Some(timer) = self.debounce.take() {
            timer.invalidate();
        }
        if let Some(timer) = self.poll.take() {
            timer.invalidate();
        }
        self.since = None;
    }

    pub fn expired(&self) -> bool {
        self.since.is_some_and(|since| since.elapsed() > MAX_WAIT)
    }
}

define_class!(
    // SAFETY: NSObject 沒有子類化要求；沒有實現 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct RescoreTicker;

    impl RescoreTicker {
        #[unsafe(method(attach:))]
        fn attach(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.attach_loaded_model());
        }

        #[unsafe(method(fire:))]
        fn fire(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.start_rescoring());
        }

        #[unsafe(method(poll:))]
        fn poll(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.poll_rescoring());
        }
    }

    unsafe impl NSObjectProtocol for RescoreTicker {}
);

impl RescoreTicker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
