//! 按鍵相關：TSF 虛擬鍵碼到協議 [`KeyEvent`](qingjian_platform::protocol::KeyEvent) 的翻譯（[`event`]）、
//! 單擊中英切換鍵的判定（[`tap`]，鍵來自 `[shortcut] switch_mode`）、「翻譯選中文字」快捷鍵的保留鍵登記（[`preserved`]）。

pub(crate) mod event;
mod layout;
pub(crate) mod preserved;
mod tap;

pub(crate) use self::tap::KeyTap;
