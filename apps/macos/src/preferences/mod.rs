//! 偏好設定視窗：配置檔案的圖形前端。
//!
//! 普通控制元件改完立即寫回 `config.toml`（`Config::set_value`，保留註釋），
//! 再經 [`crate::host::Host::apply_config`] 生效；視窗自己不存任何狀態，勾選與文本永遠從配置同步過來。
//! 自定義短語通過儲存按鈕校驗後寫入；金鑰例外：寫到配置同目錄的 `.env`，不進 `config.toml`。
//!
//! 輸入法程序是 `LSBackgroundOnly`，平時不能成為前臺應用；開啟視窗前把啟用策略臨時切成 Accessory，
//! 關窗時切回去，否則文本框拿不到鍵盤焦點。文本框裡的 ⌘C / ⌘V 靠主選單「編輯」項的快捷鍵分發，
//! 後臺應用沒有主選單，所以開窗前裝一份只有編輯項的主選單（`edit_menu`）。

mod controls;
mod edit_menu;
mod file_dialog;
mod font_picker;
mod key_recorder;
mod layout;
mod pages;
mod panel;
mod setting;
mod target;
mod window;

use objc2::runtime::AnyObject;
use objc2_app_kit::{NSButton, NSControlStateValueOn, NSPopUpButton, NSTextField};

pub use file_dialog::choose_dictionary_file;
pub use key_recorder::KeyRecorder;
pub use pages::{REPOSITORY_URL, UpdateStatus, WEBSITE_URL};
pub use setting::{Setting, SettingValue};
pub use window::PreferencesWindow;

/// 字型組合框裡代表「用系統字型」的那一項。
pub const DEFAULT_FONT_LABEL: &str = "系统默认";

/// 從 `changed:` 的 sender 認出是哪個設定、現在的值是什麼。
pub fn setting_from_sender(sender: Option<&AnyObject>) -> Option<(Setting, SettingValue)> {
    let sender = sender?;
    // 快捷鍵錄製按鈕是 NSButton 的子類，最先認它：值是它錄到的配置寫法
    if let Some(recorder) = sender.downcast_ref::<KeyRecorder>() {
        return Some((
            Setting::from_tag(recorder.tag())?,
            SettingValue::Text(recorder.recorded()),
        ));
    }
    // NSPopUpButton 是 NSButton 的子類，先認它
    if let Some(popup) = sender.downcast_ref::<NSPopUpButton>() {
        let index = usize::try_from(popup.indexOfSelectedItem()).ok()?;
        return Some((Setting::from_tag(popup.tag())?, SettingValue::Index(index)));
    }
    if let Some(button) = sender.downcast_ref::<NSButton>() {
        let on = button.state() == NSControlStateValueOn;
        return Some((Setting::from_tag(button.tag())?, SettingValue::Bool(on)));
    }
    if let Some(field) = sender.downcast_ref::<NSTextField>() {
        let text = field.stringValue().to_string();
        return Some((Setting::from_tag(field.tag())?, SettingValue::Text(text)));
    }
    None
}
