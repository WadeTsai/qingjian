//! 選單欄：「中 / 英」狀態項與輸入法選單。
//!
//! 輸入法選單是一份 NSMenu 同時掛在「中 / 英」狀態項（點選彈出）和系統輸入源選單（IMK `menu` 回撥）上。
//!
//! 菜單只是配置檔案的執行時前端：每個開關點下去先改 `config.toml` 再熱載入，不另存一套狀態；
//! 手改配置檔案與點選單走的是同一條 [`crate::host::Host::apply_config`] 通路。
//!
//! 兩處的 action 派發形態不同：狀態項選單的 sender 是 NSMenuItem 本身；輸入源選單由 IMK 轉發到
//! 控制器，sender 是帶 `IMKCommandMenuItem` 鍵的字典。[`action_from_sender`] 兩種都認，
//! 動作本身編碼在選單項的 tag 裡，所以兩邊只需要一個 `menuAction:` 選擇器。

mod action;
mod indicator;
mod menu;
mod target;

use objc2::runtime::AnyObject;
use objc2_app_kit::NSMenuItem;
use objc2_foundation::NSDictionary;
use objc2_input_method_kit::kIMKCommandMenuItemName;

pub use action::MenuAction;
pub use indicator::ModeIndicator;
pub use menu::InputMenu;

/// 從 `menuAction:` 的 sender 裡認出被點的是哪一項。
pub fn action_from_sender(sender: Option<&AnyObject>) -> Option<MenuAction> {
    let sender = sender?;
    let tag = if let Some(item) = sender.downcast_ref::<NSMenuItem>() {
        item.tag()
    } else {
        let info = sender.downcast_ref::<NSDictionary<AnyObject, AnyObject>>()?;
        // SAFETY: 只讀 IMK 匯出的常量字串
        let key: &AnyObject = unsafe { kIMKCommandMenuItemName };
        info.objectForKey(key)?.downcast::<NSMenuItem>().ok()?.tag()
    };
    MenuAction::from_tag(tag)
}
