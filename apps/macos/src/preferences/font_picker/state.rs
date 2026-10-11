//! 字型選擇器的狀態：全部字族、過濾後可見的那些、控制元件引用，供回撥獨立讀取。

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2_app_kit::{NSPopUpButton, NSPopover, NSSearchField, NSTableView};

pub(super) struct PickerState {
    /// 系統全部字族名，按系統給的順序。
    pub families: Vec<String>,

    /// 過濾後顯示的行：第 0 行永遠是「系統預設」。
    pub visible: RefCell<Vec<String>>,

    /// 配置裡當前的字族名（空為系統字型）。
    pub current: RefCell<String>,

    /// 頁面上的按鈕，顯示當前字型名。
    pub button: RefCell<Option<Retained<NSPopUpButton>>>,

    /// 下拉框裡的搜尋框。
    pub search: RefCell<Option<Retained<NSSearchField>>>,

    /// 列表。
    pub table: RefCell<Option<Retained<NSTableView>>>,

    /// 裝著列表的下拉框。
    pub popover: RefCell<Option<Retained<NSPopover>>>,

    /// 程式在同步選中行時為 true，此時選中變化不當作使用者操作。
    pub syncing: Cell<bool>,
}
