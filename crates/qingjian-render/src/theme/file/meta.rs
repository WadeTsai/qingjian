//! 主題的後設資料。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Meta {
    /// 主題 id，也是主題目錄名。
    pub(crate) id: String,

    /// 顯示名。
    pub(crate) name: String,

    #[serde(default)]
    pub(crate) author: String,

    /// SPDX 許可證標識。
    #[serde(default)]
    pub(crate) license: String,

    /// 只有一種外觀時寫 `"light"` / `"dark"`，不再跟隨外觀設定切換（背景是淺色圖片的主題配深色文字就看不清）。
    pub(crate) appearance: Option<LockedAppearance>,
}

/// 主題鎖定的外觀。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LockedAppearance {
    Light,

    Dark,
}
