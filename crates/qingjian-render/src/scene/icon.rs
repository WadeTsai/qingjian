//! 渲染器自己畫的向量小圖示。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Icon {
    /// 雲聯想的雲朵。
    Cloud,

    /// 狀態條開啟設定的齒輪。
    Gear,
}
