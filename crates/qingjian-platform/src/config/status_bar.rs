use serde::{Deserialize, Serialize};

/// `[status_bar]` 分節：桌面上常駐、可拖動的懸浮狀態條（顯示當前中 / 英，可選雙拼方案）。
/// 與工作列的中 / 英指示器（語言欄按鈕）並存，各是一條。位置記在這裡，重啟後回到原處。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusBarConfig {
    /// 是否顯示懸浮狀態條。預設關。
    pub enabled: bool,

    /// 記住的螢幕橫座標（內容左上角物理畫素）；沒拖動過為 `None`，首次按螢幕右下角擺放。
    pub x: Option<i32>,

    /// 記住的螢幕縱座標（內容左上角物理畫素）。
    pub y: Option<i32>,
}
