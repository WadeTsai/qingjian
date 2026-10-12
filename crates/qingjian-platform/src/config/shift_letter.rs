use serde::{Deserialize, Serialize};

/// 中文模式下按住 Shift 敲的字母怎麼處理（`[general] shift_letter`）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShiftLetter {
    /// 拼音原樣上屏、這個字母交給應用——兩個平臺原本的行為。預設：換掉老使用者的習慣得讓人自己選。
    #[default]
    Passthrough,

    /// 字母進組句緩衝區，按小寫參與匹配、原樣上屏（回車 / 無候選）時還原大寫，
    /// 所以 `Cpan` 與 `cpan` 一樣出「C盤」。英文模式與英文直輸段保持原樣大小寫。
    Compose,
}

impl ShiftLetter {
    /// 全部取值，設定介面按這個順序列出。
    pub const ALL: [Self; 2] = [Self::Passthrough, Self::Compose];

    /// 配置檔案裡的寫法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
            Self::Compose => "compose",
        }
    }

    /// 介面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Passthrough => "交給應用",
            Self::Compose => "進組句",
        }
    }

    /// 會不會把大寫字母收進組句緩衝區。
    pub fn compose(self) -> bool {
        matches!(self, Self::Compose)
    }
}

#[cfg(test)]
mod tests {
    use super::ShiftLetter;

    #[test]
    fn keys_round_trip_through_lowercase() {
        assert_eq!(ShiftLetter::default(), ShiftLetter::Passthrough);
        assert_eq!(ShiftLetter::Passthrough.key(), "passthrough");
        assert_eq!(ShiftLetter::Compose.key(), "compose");
        assert!(ShiftLetter::Compose.compose());
        assert!(!ShiftLetter::Passthrough.compose());
    }
}
