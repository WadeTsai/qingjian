use serde::{Deserialize, Serialize};

/// 候選視窗外觀：淺色 / 深色，或跟隨系統。與選哪個主題（`[general] theme`）無關，每個主題都有兩套顏色。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// 跟隨系統。
    #[default]
    System,

    /// 始終淺色。
    Light,

    /// 始終深色。
    Dark,
}

impl Appearance {
    /// 全部取值，設定介面按這個順序列出。
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    /// 配置檔案裡的寫法。
    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// 介面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "跟随系统",
            Self::Light => "浅色",
            Self::Dark => "深色",
        }
    }

    /// 按配置檔案裡的寫法找，不認識為 `None`。
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|appearance| appearance.key() == key)
    }
}
