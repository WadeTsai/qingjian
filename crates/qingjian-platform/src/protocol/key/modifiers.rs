use serde::{Deserialize, Serialize};

use crate::config::Modifiers;

/// 一次按鍵按下時的修飾鍵狀態（Windows 語義）。後兩位不是物理修飾鍵，是輸入法狀態：
/// `caps` 是 Caps Lock 鎖定位（管大小寫、亮著即臨時英文大寫），`english_mode` 是 Shift 單擊切出來的持久中英模式。
///
/// 普通字元鍵通常都是 `false`，能幹淨序列化——不像配置層的 `Modifiers`（走字串、空值不合法，
/// 那個是給快捷鍵配置用的）。協議要能表達「沒有修飾鍵」，所以自帶這個而不復用它。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    /// Ctrl
    pub ctrl: bool,

    /// Shift
    pub shift: bool,

    /// Alt
    pub alt: bool,

    /// Win（⊞）
    pub win: bool,

    /// Caps Lock 亮著（鎖定狀態，不是按著）。管字母大小寫；亮著時無論中英文模式都直接出大寫英文（微軟拼音式）。
    #[serde(default)]
    pub caps: bool,

    /// 持久的英文模式（Windows 單擊 Shift 切換，DLL 記狀態）。中文模式為 `false`。macOS 殼不用這個欄位。
    #[serde(default)]
    pub english_mode: bool,
}

impl KeyModifiers {
    /// 按著的物理修飾鍵組合（去掉 Caps Lock 與中英模式這兩個輸入法狀態位），拿來與配置的快捷鍵比。
    pub fn chord(self) -> Self {
        Self {
            caps: false,
            english_mode: false,
            ..self
        }
    }

    /// 有沒有按著 Ctrl / Alt / Win（Shift 不算：它只改字元大小寫與標點）。
    pub fn has_command_key(self) -> bool {
        self.ctrl || self.alt || self.win
    }
}

/// 配置裡的修飾鍵組合按 macOS 命名（option / command），落到 Windows 鍵位：⌥ 是 Alt、⌃ 是 Ctrl、⌘ 是 Win。
impl From<Modifiers> for KeyModifiers {
    fn from(modifiers: Modifiers) -> Self {
        Self {
            ctrl: modifiers.control,
            shift: modifiers.shift,
            alt: modifiers.option,
            win: modifiers.command,
            caps: false,
            english_mode: false,
        }
    }
}
