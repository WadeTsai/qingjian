use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

/// 一箇中 / 英切換鍵（Windows）。`shift` / `control` 是**單擊**那個修飾鍵；`ctrl+alt+space` 是組合鍵
/// （走 TSF 保留鍵登記，與「翻譯選中文字」同一套機制）。macOS 的切換鍵是 Caps Lock，本項不生效。
///
/// 不用 Ctrl + Space：中文 Windows 把它綁成系統的「輸入法/非輸入法切換」，系統先截走，輸入法拿不到。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchKey {
    /// 單擊 Shift（預設）。與微軟拼音一致，但打字時容易誤觸。
    Shift,

    /// 單擊 Ctrl：Shift 老是誤觸時換它。
    Control,

    /// Ctrl + Alt + Space 組合鍵，最不容易誤觸。
    CtrlAltSpace,
}

impl SwitchKey {
    /// 全部取值，設定介面按這個順序列出。
    pub const ALL: [Self; 3] = [Self::Shift, Self::Control, Self::CtrlAltSpace];

    /// 配置檔案裡的寫法。
    pub const fn key(self) -> &'static str {
        match self {
            Self::Shift => "shift",
            Self::Control => "control",
            Self::CtrlAltSpace => "ctrl+alt+space",
        }
    }

    /// 介面上的名字。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Shift => "單擊 Shift",
            Self::Control => "單擊 Ctrl",
            Self::CtrlAltSpace => "Ctrl + Alt + Space",
        }
    }

    /// 讀配置寫法（含別名）。老配置的 `ctrl+space` 換成 Ctrl + Alt + Space。
    fn parse(text: &str) -> Option<Self> {
        match text {
            "shift" => Some(Self::Shift),
            "control" | "ctrl" => Some(Self::Control),
            "ctrl+alt+space" | "control+alt+space" | "ctrl+space" | "control+space" => {
                Some(Self::CtrlAltSpace)
            }
            _ => None,
        }
    }
}

/// 勾選了哪些中 / 英切換鍵（`[shortcut] switch_mode`），可以多選，一個都不勾就只剩按鈕能切。
///
/// 配置裡寫成列表 `["shift", "control"]`；老配置的單個字串（`"shift"`、`"none"`）照樣讀。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchKeys {
    pub shift: bool,
    pub control: bool,
    pub ctrl_alt_space: bool,
}

impl Default for SwitchKeys {
    /// 預設只勾單擊 Shift。
    fn default() -> Self {
        Self::NONE.with(SwitchKey::Shift, true)
    }
}

impl SwitchKeys {
    /// 一個都不勾。
    pub const NONE: Self = Self {
        shift: false,
        control: false,
        ctrl_alt_space: false,
    };

    pub const fn contains(self, key: SwitchKey) -> bool {
        match key {
            SwitchKey::Shift => self.shift,
            SwitchKey::Control => self.control,
            SwitchKey::CtrlAltSpace => self.ctrl_alt_space,
        }
    }

    /// 勾上 / 去掉一個鍵。
    pub const fn with(mut self, key: SwitchKey, on: bool) -> Self {
        match key {
            SwitchKey::Shift => self.shift = on,
            SwitchKey::Control => self.control = on,
            SwitchKey::CtrlAltSpace => self.ctrl_alt_space = on,
        }
        self
    }

    /// 勾著的鍵，按 [`SwitchKey::ALL`] 的順序。
    pub fn keys(self) -> impl Iterator<Item = SwitchKey> {
        SwitchKey::ALL
            .into_iter()
            .filter(move |key| self.contains(*key))
    }

    /// 配置寫法的列表（設定程式寫回配置用）。
    pub fn config_values(self) -> Vec<&'static str> {
        self.keys().map(SwitchKey::key).collect()
    }

    /// 日誌與提示用：`單擊 Shift、單擊 Ctrl`，一個都沒勾時 `無`。
    pub fn describe(self) -> String {
        let labels: Vec<&str> = self.keys().map(SwitchKey::label).collect();
        if labels.is_empty() {
            "無".to_owned()
        } else {
            labels.join("、")
        }
    }
}

impl Serialize for SwitchKeys {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let values = self.config_values();
        let mut seq = serializer.serialize_seq(Some(values.len()))?;
        for value in values {
            seq.serialize_element(value)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for SwitchKeys {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(SwitchKeysVisitor)
    }
}

struct SwitchKeysVisitor;

impl SwitchKeysVisitor {
    fn add<E: de::Error>(keys: SwitchKeys, text: &str) -> Result<SwitchKeys, E> {
        SwitchKey::parse(text)
            .map(|key| keys.with(key, true))
            .ok_or_else(|| E::custom(format!("未知的中英切換鍵「{text}」")))
    }
}

impl<'de> Visitor<'de> for SwitchKeysVisitor {
    type Value = SwitchKeys;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("切換鍵列表，如 [\"shift\", \"control\"]")
    }

    /// 老配置的單個值。
    fn visit_str<E: de::Error>(self, text: &str) -> Result<SwitchKeys, E> {
        match text {
            "none" | "off" | "disabled" => Ok(SwitchKeys::NONE),
            _ => Self::add(SwitchKeys::NONE, text),
        }
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<SwitchKeys, A::Error> {
        let mut keys = SwitchKeys::NONE;
        while let Some(text) = seq.next_element::<String>()? {
            keys = Self::add(keys, &text)?;
        }
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Wrapper {
        k: SwitchKeys,
    }

    fn parse(value: &str) -> Result<SwitchKeys, toml::de::Error> {
        toml::from_str::<Wrapper>(&format!("k = {value}")).map(|wrapper| wrapper.k)
    }

    #[test]
    fn reads_lists_and_old_single_values() {
        let shift = SwitchKeys::NONE.with(SwitchKey::Shift, true);
        let both = shift.with(SwitchKey::Control, true);
        assert_eq!(parse(r#"["shift", "ctrl"]"#).unwrap(), both);
        assert_eq!(parse(r#"["control", "shift"]"#).unwrap(), both);
        assert_eq!(parse("[]").unwrap(), SwitchKeys::NONE);
        assert_eq!(parse(r#""shift""#).unwrap(), shift);
        assert_eq!(parse(r#""none""#).unwrap(), SwitchKeys::NONE);
        // 老配置的 Ctrl + Space 換成 Ctrl + Alt + Space
        let chord = SwitchKeys::NONE.with(SwitchKey::CtrlAltSpace, true);
        assert_eq!(parse(r#""ctrl+space""#).unwrap(), chord);
        assert_eq!(parse(r#"["ctrl+alt+space"]"#).unwrap(), chord);
        assert_eq!(SwitchKeys::default(), shift);
    }

    #[test]
    fn unknown_values_are_rejected() {
        assert!(parse(r#""hyper""#).is_err());
        assert!(parse(r#"["shift", "hyper"]"#).is_err());
    }

    #[test]
    fn writes_a_list_in_display_order() {
        let keys = SwitchKeys::NONE
            .with(SwitchKey::CtrlAltSpace, true)
            .with(SwitchKey::Shift, true);
        assert_eq!(keys.config_values(), ["shift", "ctrl+alt+space"]);
        assert_eq!(keys.describe(), "單擊 Shift、Ctrl + Alt + Space");
        assert_eq!(SwitchKeys::NONE.describe(), "無");
        let json = serde_json::to_string(&keys).unwrap();
        assert_eq!(json, r#"["shift","ctrl+alt+space"]"#);
        assert_eq!(serde_json::from_str::<SwitchKeys>(&json).unwrap(), keys);
    }
}
