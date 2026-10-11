use qingjian_core::ModeKeys;
use serde::{Deserialize, Serialize};

use super::key_combo::KeyCombo;
use super::modifiers::Modifiers;
use super::switch_key::SwitchKeys;

/// 配置檔案 `[shortcut]` 分節：字首模式鍵（Core 的 [`ModeKeys`]）加殼層的修飾鍵組合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutConfig {
    /// 表示式 / 問字模式鍵，鍵名與以前一樣直接在分節下（`expression` / `question`）。
    #[serde(flatten)]
    pub mode: ModeKeys,

    /// 中 / 英切換鍵（Windows 用），可多選：`["shift", "control", "ctrl+alt+space"]`。詳見 [`SwitchKeys`]。
    pub switch_mode: SwitchKeys,

    /// 數字鍵配這些修飾鍵：上屏候選的第一個譯詞。
    pub translation: Modifiers,

    /// 數字鍵配這些修飾鍵：上屏候選的第二個譯詞（候選右側有兩個譯詞時）。
    pub translation_second: Modifiers,

    /// 把應用裡選中的文字譯成學習語言（需要雲服務開著）。
    pub translate_selection: KeyCombo,

    /// 數字鍵配這些修飾鍵：刪掉候選（使用者詞整個刪掉，詞庫詞清掉對它的學習）。
    pub delete_candidate: Modifiers,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        // Windows 上 Alt+數字被系統當選單快捷鍵截走（TSF 收不到），譯詞鍵預設用 Ctrl；macOS 用 Option。
        #[cfg(windows)]
        let (translation, translation_second) = (Modifiers::CONTROL, Modifiers::SHIFT_CONTROL);
        #[cfg(not(windows))]
        let (translation, translation_second) = (Modifiers::OPTION, Modifiers::SHIFT_OPTION);
        Self {
            mode: ModeKeys::default(),
            switch_mode: SwitchKeys::default(),
            translation,
            translation_second,
            translate_selection: KeyCombo::TRANSLATE_DEFAULT,
            delete_candidate: Modifiers::SHIFT,
        }
    }
}

impl ShortcutConfig {
    /// 刪候選的修飾鍵；為空或與任一組譯詞鍵撞了就退回預設。
    pub fn delete_keys(&self) -> Modifiers {
        let (first, second) = self.translation_keys();
        if self.delete_candidate.is_empty()
            || self.delete_candidate == first
            || self.delete_candidate == second
        {
            Self::default().delete_candidate
        } else {
            self.delete_candidate
        }
    }

    /// 兩組譯詞修飾鍵；兩組相同或有一組為空時整個退回預設，不做一半。
    pub fn translation_keys(&self) -> (Modifiers, Modifiers) {
        if self.translation == self.translation_second
            || self.translation.is_empty()
            || self.translation_second.is_empty()
        {
            let default = Self::default();
            (default.translation, default.translation_second)
        } else {
            (self.translation, self.translation_second)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_files_without_modifier_keys_still_parse_and_get_defaults() {
        // 預設值分平臺（Windows 用 Ctrl 系、其餘用 Option 系），斷言跟著平臺的 Default 走
        let default = ShortcutConfig::default();
        let parsed: ShortcutConfig = toml::from_str("expression = \"i\"\n").unwrap();
        assert_eq!(parsed.mode.expression, 'i');
        assert_eq!(parsed.switch_mode, SwitchKeys::default());
        assert_eq!(
            parsed.translation_keys(),
            (default.translation, default.translation_second)
        );
        let same: ShortcutConfig =
            toml::from_str("translation = \"option\"\ntranslation_second = \"option\"\n").unwrap();
        assert_eq!(
            same.translation_keys(),
            (default.translation, default.translation_second)
        );
        let swapped: ShortcutConfig =
            toml::from_str("translation = \"control+option\"\ntranslation_second = \"option\"\n")
                .unwrap();
        assert_eq!(swapped.translation_keys().1, Modifiers::OPTION);
    }

    #[test]
    fn delete_keys_fall_back_when_clashing_with_translation_keys() {
        let default = ShortcutConfig::default();
        let parsed: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(parsed.delete_keys(), default.delete_candidate);
        // 與平臺預設的譯詞鍵撞上才算「衝突」，兩邊平臺都成立
        let clash: ShortcutConfig = toml::from_str(&format!(
            "delete_candidate = \"{}\"\n",
            default.translation.key()
        ))
        .unwrap();
        assert_eq!(clash.delete_keys(), default.delete_candidate);
        // 不與任何一組譯詞鍵衝突的修飾鍵：平臺上取一個，斷言它原樣生效
        let free = [Modifiers::OPTION, Modifiers::CONTROL, Modifiers::SHIFT]
            .into_iter()
            .find(|m| *m != default.translation && *m != default.translation_second)
            .unwrap();
        let custom: ShortcutConfig =
            toml::from_str(&format!("delete_candidate = \"{}\"\n", free.key())).unwrap();
        assert_eq!(custom.delete_keys(), free);
    }

    #[test]
    fn switch_mode_parses_and_defaults_to_shift() {
        let parsed: ShortcutConfig = toml::from_str("switch_mode = [\"ctrl\"]\n").unwrap();
        assert!(parsed.switch_mode.control && !parsed.switch_mode.shift);
        let off: ShortcutConfig = toml::from_str("switch_mode = \"none\"\n").unwrap();
        assert_eq!(off.switch_mode, crate::SwitchKeys::NONE);
        let missing: ShortcutConfig = toml::from_str("").unwrap();
        assert_eq!(missing.switch_mode, SwitchKeys::default());
    }
}
