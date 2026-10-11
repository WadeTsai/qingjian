//! 配置檔案監視：輸入法啟用期間每秒看一眼 `config.toml` 的修改時間，變了就熱載入。
//! 一次 `stat` 的開銷可以忽略，不值得為此引 FSEvents。同一個定時器順帶定期把學習資料落盤（`Host::tick`）。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 檢查間隔（秒）。
const POLL_INTERVAL: f64 = 1.0;

pub struct ConfigWatch {
    /// 定時器；未啟用時為 `None`。
    timer: Option<Retained<NSTimer>>,

    mtm: MainThreadMarker,
}

impl ConfigWatch {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self { timer: None, mtm }
    }

    pub fn start(&mut self) {
        if self.timer.is_some() {
            return;
        }
        let target = ConfigWatchTick::new(self.mtm);
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
    }
}

define_class!(
    // SAFETY: NSObject 沒有子類化要求；沒有實現 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct ConfigWatchTick;

    impl ConfigWatchTick {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            // 定時器回撥也是 ObjC 執行時直接調的，panic 同樣不能穿出去
            if crate::imk::catch_panic("定时器", || crate::host::with(|h| h.tick())).is_none() {
                crate::imk::recover_from_panic(None);
            }
        }
    }

    unsafe impl NSObjectProtocol for ConfigWatchTick {}
);

impl ConfigWatchTick {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
