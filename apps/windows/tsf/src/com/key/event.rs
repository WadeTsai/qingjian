//! 把 TSF 送來的虛擬鍵碼翻成協議的 [`KeyEvent`]，以及「組句中哪些鍵要吃」的判定。

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VIRTUAL_KEY, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_ESCAPE, VK_LWIN, VK_MENU,
    VK_RETURN, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
};

use qingjian_platform::protocol::{KeyEvent, KeyModifiers};

/// 採當前修飾鍵並解析字元（標點 / 數字使用當前鍵盤佈局）。`english_mode` 是 DLL 記的持久中英模式，隨事件帶給 Server。
pub(crate) fn to_key_event(vk: u32, english_mode: bool) -> KeyEvent {
    let modifiers = current_modifiers(english_mode);
    KeyEvent::new(
        vk,
        resolve_char(vk, modifiers.shift, modifiers.caps),
        modifiers,
    )
}

pub(crate) fn is_letter(vk: u32) -> bool {
    (0x41..=0x5A).contains(&vk)
}

/// 可配成模式鍵的字母 V / U / I（Core `ModeKeys::CANDIDATES`）：雙拼下按住 Shift 是表示式 / 問字入口。
pub(crate) fn is_mode_letter(vk: u32) -> bool {
    matches!(vk, 0x56 | 0x55 | 0x49)
}

/// 組句中要吃的功能鍵：退格 / Tab / 回車 / Esc / 空格 / 數字。Tab 由 Router 決定接受整句補全或翻頁，Shift+Tab 上一頁。
pub(crate) fn is_edit(vk: u32) -> bool {
    matches!(
        VIRTUAL_KEY(vk as u16),
        VK_BACK | VK_TAB | VK_RETURN | VK_ESCAPE | VK_SPACE
    ) || is_digit(vk)
}

/// PageUp/Down、End、Home、方向鍵。
pub(crate) fn is_nav(vk: u32) -> bool {
    (0x21..=0x28).contains(&vk)
}

fn is_digit(vk: u32) -> bool {
    (0x30..=0x39).contains(&vk)
}

/// 主鍵盤區 1–9（修飾鍵 + 數字的快捷鍵按這個認）。
pub(crate) fn digit_key(vk: u32) -> bool {
    (0x31..=0x39).contains(&vk)
}

fn current_modifiers(english_mode: bool) -> KeyModifiers {
    KeyModifiers {
        ctrl: key_down(VK_CONTROL),
        shift: key_down(VK_SHIFT),
        alt: key_down(VK_MENU),
        win: key_down(VK_LWIN) || key_down(VK_RWIN),
        caps: key_toggled(VK_CAPITAL),
        english_mode,
    }
}

/// 高位為 1（返回值為負）表示按下。
fn key_down(vk: VIRTUAL_KEY) -> bool {
    let state = unsafe { GetKeyState(vk.0 as i32) };
    state < 0
}

pub(crate) fn caps_lock_on() -> bool {
    key_toggled(VK_CAPITAL)
}

/// 低位為 1 表示鎖定鍵亮著。
fn key_toggled(vk: VIRTUAL_KEY) -> bool {
    let state = unsafe { GetKeyState(vk.0 as i32) };
    state & 1 != 0
}

/// 字母大小寫 = Shift 異或 Caps；數字 / 標點使用系統佈局；功能鍵 `None`。
fn resolve_char(vk: u32, shift: bool, caps: bool) -> Option<char> {
    if is_letter(vk) {
        let lower = (b'a' + (vk - 0x41) as u8) as char;
        return Some(if shift != caps {
            lower.to_ascii_uppercase()
        } else {
            lower
        });
    }
    super::layout::character(vk)
}
