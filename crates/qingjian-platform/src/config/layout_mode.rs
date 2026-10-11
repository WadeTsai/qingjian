use serde::{Deserialize, Serialize};

/// 候選視窗排布。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    /// 豎排：一行一個候選，譯文在同一行右側。
    #[default]
    Vertical,

    /// 橫排：候選排成一行，只給高亮那個在下面顯示譯文。
    Horizontal,
}

impl LayoutMode {
    /// 全部取值，設定介面按這個順序列出。
    pub const ALL: [Self; 2] = [Self::Vertical, Self::Horizontal];

    /// 配置檔案裡的寫法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        }
    }

    /// 介面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Vertical => "竖排",
            Self::Horizontal => "横排",
        }
    }
}
