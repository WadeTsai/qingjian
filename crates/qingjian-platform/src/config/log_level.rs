use serde::{Deserialize, Serialize};

/// 日誌級別。預設 info 不含使用者敲的內容；debug 會把敲的拼音與上屏的文字記進日誌，只在配合排查問題時開。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// 只記啟動、載入、錯誤與配置變化。
    #[default]
    Info,

    /// 逐鍵、逐次上屏都記。
    Debug,
}

impl LogLevel {
    /// 全部取值，設定介面按這個順序列出。
    pub const ALL: [Self; 2] = [Self::Info, Self::Debug];

    /// 介面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "簡略（info）",
            Self::Debug => "詳細（debug）",
        }
    }

    /// 配置檔案裡的寫法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}
