use serde::{Deserialize, Serialize};

/// 工作列「中 / 英」圖示右鍵選單裡要交給 Server 辦的項（中 / 英切換在 DLL 側自己做）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndicatorCommand {
    /// 翻轉當前模式的全形標點，與懸浮條上點「，。」一樣。
    TogglePunctuation,

    /// 顯示 / 隱藏懸浮狀態條（`[status_bar] enabled`）。
    ToggleStatusBar,

    /// 開啟設定程式。DLL 可能在 UWP 沙箱裡起不了程序，交給 Server 起。
    OpenSettings,

    /// 查到新版本時選單裡的「有新版本」：開啟下載頁，同樣交給 Server。
    OpenDownload,
}

/// 右鍵選單打勾用的開關狀態。DLL 不讀配置檔案（UWP 沙箱裡讀不到），由 Server 隨
/// [`super::ServerMessage::ModeSync`] 每一拍帶下來。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct IndicatorState {
    /// 中文模式下標點轉全形（`[general] full_width_punctuation`）。
    pub full_width_punctuation: bool,

    /// 英文模式下標點轉全形（`[general] english_full_width_punctuation`）。
    pub english_full_width_punctuation: bool,

    /// 懸浮狀態條開著（`[status_bar] enabled`）。
    pub status_bar: bool,

    /// 檢查更新查到了新版本，選單裡露出「有新版本」。
    pub update_available: bool,
}
