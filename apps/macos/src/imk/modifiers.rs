//! 當前修飾鍵狀態。IMK 的 `inputText:client:` 不帶事件物件，Caps Lock / Shift 只能從系統當前狀態讀。

use objc2_app_kit::{NSEvent, NSEventModifierFlags};

/// Caps Lock 亮著：視為英文模式，字母預設小寫、按住 Shift 才大寫、標點不轉全形。
pub fn caps_lock_on() -> bool {
    NSEvent::modifierFlags_class().contains(NSEventModifierFlags::CapsLock)
}

/// Shift 正按著。讀的是此刻的硬體狀態而不是事件自帶的標誌，但 Shift 是按住不放的鍵，處理按鍵時它幾乎總還按著。
/// macOS 上 Caps Lock 亮著時按住 Shift 送來的仍是大寫（不像 Windows 會反轉），所以英文模式的大小寫只能靠它判斷。
pub fn shift_down() -> bool {
    NSEvent::modifierFlags_class().contains(NSEventModifierFlags::Shift)
}
