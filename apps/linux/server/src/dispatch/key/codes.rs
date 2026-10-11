//! 按鍵分派用的虛擬鍵碼與字元解析。

use qingjian_platform::protocol::KeyEvent;

pub(crate) const BACK: u32 = 0x08;
pub(crate) const DELETE: u32 = 0x2E;
pub(crate) const TAB: u32 = 0x09;
pub(crate) const RETURN: u32 = 0x0D;
pub(crate) const ESCAPE: u32 = 0x1B;
pub(crate) const PRIOR: u32 = 0x21;
pub(crate) const NEXT: u32 = 0x22;
pub(crate) const END: u32 = 0x23;
pub(crate) const HOME: u32 = 0x24;
pub(crate) const LEFT: u32 = 0x25;
pub(crate) const UP: u32 = 0x26;
pub(crate) const RIGHT: u32 = 0x27;
pub(crate) const DOWN: u32 = 0x28;

/// 翻頁鍵對 `(上一頁, 下一頁)`：返回 -1 / +1。
pub(crate) fn page_key(event: &KeyEvent, page_keys: (char, char)) -> Option<isize> {
    let c = event.character?;
    if c == page_keys.0 {
        Some(-1)
    } else if c == page_keys.1 {
        Some(1)
    } else {
        None
    }
}

/// 敲出來是數字 1–9 的鍵（選候選用）：按 `character` 認，Shift 出的 `!@#` 不算。
pub(crate) fn digit(event: &KeyEvent) -> Option<usize> {
    match event.character {
        Some(c) => ('1'..='9').contains(&c).then(|| c as usize - '0' as usize),
        None => digit_key(event.virtual_key),
    }
}

/// 主鍵盤區數字鍵 1–9 的鍵碼，不管修飾鍵（修飾鍵 + 數字的快捷鍵按鍵位認）。
pub(crate) fn digit_key(virtual_key: u32) -> Option<usize> {
    (0x31..=0x39)
        .contains(&virtual_key)
        .then(|| (virtual_key - 0x30) as usize)
}

/// 協議保留小鍵盤來源，運算子保持半形。
pub(crate) fn is_keypad(virtual_key: u32) -> bool {
    (0x60..=0x6F).contains(&virtual_key)
}
