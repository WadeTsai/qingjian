use serde::{Deserialize, Serialize};

use super::modifiers::KeyModifiers;

/// DLL 從 TSF `OnKeyDown` / `OnTestKeyDown` 抓到的一次按鍵，發給 Server 判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEvent {
    /// Windows 虛擬鍵碼（`VK_*`）。翻頁、方向鍵、退格、回車等靠它區分。
    pub virtual_key: u32,

    /// 這次按鍵產生的字元（`ToUnicode` 的結果）；功能鍵沒有字元時為 `None`。
    pub character: Option<char>,

    /// 按下時的修飾鍵狀態。
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    pub fn new(virtual_key: u32, character: Option<char>, modifiers: KeyModifiers) -> Self {
        Self {
            virtual_key,
            character,
            modifiers,
        }
    }
}
