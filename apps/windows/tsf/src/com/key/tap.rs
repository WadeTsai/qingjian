//! 單擊中 / 英切換鍵的判定，喂的是擊鍵 sink 的 `OnTestKeyDown` / `OnTestKeyUp`（被吃掉的鍵也經過它們，
//! 與 `WH_KEYBOARD` 鉤子不同）。按下切換鍵到抬起之間沒插進別的鍵，就是一次單擊。
//!
//! 切換鍵來自 `[shortcut] switch_mode`，單擊 Shift / 單擊 Ctrl 可以都勾；Ctrl + Alt + Space 是組合鍵，走保留鍵。
//! 系統熱鍵（如 Ctrl + Space）的第二個鍵被系統截走、到不了這裡，看起來就像單擊了 Ctrl：
//! 系統熱鍵生效時調 [`KeyTap::cancel`] 作廢這次按下。

use std::cell::Cell;

use windows::Win32::Foundation::LPARAM;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_LCONTROL, VK_LSHIFT, VK_RCONTROL, VK_RSHIFT, VK_SHIFT,
};

use qingjian_platform::{SwitchKey, SwitchKeys};

#[derive(Default)]
pub(crate) struct KeyTap {
    /// 按下了哪個切換鍵、之後還沒有別的鍵插進來。
    pressed: Cell<Option<SwitchKey>>,
}

impl KeyTap {
    /// 任一鍵按下。`lparam` 第 30 位是按下前的狀態（1 = 自動重複，不算新按下）。
    pub(crate) fn key_down(&self, vk: u32, lparam: LPARAM, keys: SwitchKeys) {
        let Some(key) = tap_key(keys, vk) else {
            self.cancel();
            return;
        };
        if (lparam.0 >> 30) & 1 != 0 {
            return;
        }
        // 兩個切換鍵一起按（Ctrl + Shift 是系統換佈局的鍵）不算單擊
        let pressed = match self.pressed.get() {
            Some(other) if other != key => None,
            _ => Some(key),
        };
        self.pressed.set(pressed);
    }

    /// 任一鍵抬起；切換鍵單獨抬起返回 `true`，一次抬起只算一次。
    pub(crate) fn key_up(&self, vk: u32, keys: SwitchKeys) -> bool {
        let Some(key) = tap_key(keys, vk) else {
            return false;
        };
        if self.pressed.get() == Some(key) {
            self.pressed.set(None);
            true
        } else {
            false
        }
    }

    /// 作廢正按著的切換鍵（按下之後發生了別的事，這次抬起不算單擊）。
    pub(crate) fn cancel(&self) {
        self.pressed.set(None);
    }
}

/// 這個虛擬鍵碼是不是勾著的單擊切換鍵（左右兩個都算）；組合鍵 Ctrl + Alt + Space 不走單擊判定。
fn tap_key(keys: SwitchKeys, vk: u32) -> Option<SwitchKey> {
    let is = |codes: [u16; 3]| codes.iter().any(|code| u32::from(*code) == vk);
    if keys.shift && is([VK_SHIFT.0, VK_LSHIFT.0, VK_RSHIFT.0]) {
        Some(SwitchKey::Shift)
    } else if keys.control && is([VK_CONTROL.0, VK_LCONTROL.0, VK_RCONTROL.0]) {
        Some(SwitchKey::Control)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按下再抬起的 lparam（第 30 位為 0 表示新按下）。
    const DOWN: LPARAM = LPARAM(0);
    const VK_SHIFT_LEFT: u32 = 0xA0;
    const VK_CONTROL_LEFT: u32 = 0xA2;

    fn only(key: SwitchKey) -> SwitchKeys {
        SwitchKeys::NONE.with(key, true)
    }

    #[test]
    fn shift_tap_fires_only_when_nothing_else_interrupts() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Shift);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_SHIFT_LEFT, keys));
        // 一次抬起只算一次
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));

        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        tap.key_down(0x41, DOWN, keys); // 中間插了一個 A
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
    }

    #[test]
    fn only_checked_keys_fire() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Control);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_CONTROL_LEFT, keys));

        // 一個都不勾、只勾組合鍵：修飾鍵單擊都不算
        for keys in [SwitchKeys::NONE, only(SwitchKey::CtrlAltSpace)] {
            tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
            assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
            tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
            assert!(!tap.key_up(VK_CONTROL_LEFT, keys));
        }
    }

    #[test]
    fn both_taps_work_when_both_are_checked_but_not_together() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Shift).with(SwitchKey::Control, true);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_SHIFT_LEFT, keys));
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_CONTROL_LEFT, keys));

        // Ctrl + Shift 一起按：誰抬起都不算
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
        assert!(!tap.key_up(VK_CONTROL_LEFT, keys));
    }

    #[test]
    fn a_system_hotkey_cancels_the_pending_tap() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Control);
        // Ctrl + Space 的 Space 被系統截走，這裡只看到 Ctrl 按下又抬起
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        tap.cancel();
        assert!(!tap.key_up(VK_CONTROL_LEFT, keys));
    }

    #[test]
    fn auto_repeat_does_not_rearm() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Shift);
        // 第 30 位為 1：自動重複，不算新按下
        let repeat = LPARAM(1 << 30);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert!(tap.key_up(VK_SHIFT_LEFT, keys));
        tap.key_down(VK_SHIFT_LEFT, repeat, keys);
        assert!(!tap.key_up(VK_SHIFT_LEFT, keys));
    }
}
