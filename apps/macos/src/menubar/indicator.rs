//! 選單欄裡的「中 / 英」狀態項。
//!
//! 輸入源圖示（Info.plist 的 tsInputMethodIconFileKey）沒法動態換，所以自己放一個 NSStatusItem。
//! Caps Lock 的變化不會作為按鍵送到輸入法，用一個定時器輪詢系統狀態重新整理。
//!
//! 狀態項一旦建立就**不再隱藏**：`setVisible(false)` 再 `setVisible(true)` 會把它重新排到選單欄最左邊，使用者 ⌘ 拖到輸入法圖示旁的位置就丟了
//! （固定 autosave 名也保不住），而焦點每進出一次輸入框 IMK 就 deactivate / activate 一輪。
//! 停用時改成收成零寬、清空標題，並且延遲 [`COLLAPSE_DELAY`] 再收：焦點只是在輸入框之間挪的話，半秒內就會再次啟用，根本收不下去；
//! 真換到別的輸入法才收起來，切回來再展開，位置一直在。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSMenu, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::{NSObject, NSObjectProtocol, NSString, NSTimer, ns_string};

use crate::imk::modifiers;

/// 輪詢 Caps Lock 狀態的間隔。
const POLL_INTERVAL: f64 = 0.25;

/// 停用後隔多久才把狀態項收起：焦點在輸入框之間挪動時 deactivate 與下一次 activate 只隔幾十毫秒。
const COLLAPSE_DELAY: f64 = 0.5;

pub struct ModeIndicator {
    /// 選單欄狀態項。
    item: Retained<NSStatusItem>,

    /// 輪詢定時器；未啟用時為 `None`。
    timer: Option<Retained<NSTimer>>,

    /// 停用後延遲收起的一次性定時器；再次啟用時取消。
    collapse_timer: Option<Retained<NSTimer>>,

    /// 正展開著（輸入法啟用中）。收起時不重新整理標題。
    shown: bool,

    /// 上次顯示的是否英文模式，避免每次輪詢都重設標題。
    english: Option<bool>,

    /// 雲聯想開著：標題帶雲朵，讓使用者一眼知道上下文會發出去。
    cloud: bool,

    mtm: MainThreadMarker,
}

impl ModeIndicator {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(0.0);
        item.setAutosaveName(Some(ns_string!("QingjianModeIndicator")));
        item.setVisible(true);
        Self {
            item,
            timer: None,
            collapse_timer: None,
            shown: false,
            english: None,
            cloud: false,
            mtm,
        }
    }

    /// 輸入法啟用：展開狀態項並開始輪詢；停用時安排的收起取消。
    pub fn activate(&mut self) {
        if let Some(timer) = self.collapse_timer.take() {
            timer.invalidate();
        }
        if !self.shown {
            self.shown = true;
            self.item.setLength(NSVariableStatusItemLength);
        }
        self.english = None;
        self.update();
        if self.timer.is_none() {
            let target = ModeMonitor::new(self.mtm);
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
    }

    /// 輸入法停用：停止輪詢，半秒後沒再啟用就收起。
    pub fn deactivate(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
        if self.collapse_timer.is_some() {
            return;
        }
        let target = ModeMonitor::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                COLLAPSE_DELAY,
                &target,
                sel!(collapse:),
                None,
                false,
            )
        };
        self.collapse_timer = Some(timer);
    }

    /// 收成零寬、清空標題；位置保留。
    pub fn collapse(&mut self) {
        self.collapse_timer = None;
        if !self.shown {
            return;
        }
        self.shown = false;
        self.english = None;
        if let Some(button) = self.item.button(self.mtm) {
            button.setTitle(ns_string!(""));
        }
        self.item.setLength(0.0);
    }

    /// 點狀態項彈出的選單。
    pub fn set_menu(&self, menu: &NSMenu) {
        self.item.setMenu(Some(menu));
    }

    pub fn set_cloud(&mut self, cloud: bool) {
        self.cloud = cloud;
        self.english = None;
    }

    /// 按當前 Caps Lock 狀態重新整理標題；收起時不動。
    pub fn update(&mut self) {
        if !self.shown {
            return;
        }
        let english = modifiers::caps_lock_on();
        if self.english == Some(english) {
            return;
        }
        self.english = Some(english);
        if let Some(button) = self.item.button(self.mtm) {
            let mode = if english { "英" } else { "中" };
            let title = if self.cloud {
                format!("{mode} ☁︎")
            } else {
                mode.to_owned()
            };
            button.setTitle(&NSString::from_str(&title));
        }
    }
}

define_class!(
    // SAFETY: NSObject 沒有子類化要求；沒有實現 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct ModeMonitor;

    impl ModeMonitor {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.indicator.update());
        }

        #[unsafe(method(collapse:))]
        fn collapse(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.indicator.collapse());
        }
    }

    unsafe impl NSObjectProtocol for ModeMonitor {}
);

impl ModeMonitor {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
