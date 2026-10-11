//! 自定義短語表格的快照與控制元件引用，供表格回撥獨立讀取。

use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSTableView};
use qingjian_core::CustomPhrase;
use std::cell::RefCell;

pub(super) struct TableState {
    /// 當前配置的只讀顯示副本，回撥不借用 Host。
    pub rows: RefCell<Vec<CustomPhrase>>,

    /// 所屬表格。
    pub table: RefCell<Option<Retained<NSTableView>>>,

    /// 無選中行時停用編輯。
    pub edit: Retained<NSButton>,

    /// 無選中行時停用刪除。
    pub delete: Retained<NSButton>,
}
