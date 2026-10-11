use serde::{Deserialize, Serialize};

/// 更新渠道。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    /// 只看正式版。
    #[default]
    Stable,

    /// 正式版與測試版（alpha / beta / rc）裡最新的那個。
    Beta,
}

impl UpdateChannel {
    /// 全部取值，設定介面按這個順序列出。
    pub const ALL: [Self; 2] = [Self::Stable, Self::Beta];

    /// 配置檔案裡的寫法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
        }
    }

    /// 介面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Stable => "正式版",
            Self::Beta => "测试版",
        }
    }

    /// 索引裡標著 `release_channel` 的版本在這個渠道里看不看得到。
    pub fn includes(self, release_channel: &str) -> bool {
        match self {
            Self::Stable => release_channel == "stable",
            Self::Beta => true,
        }
    }
}
