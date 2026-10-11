//! 狀態條一格的動作。

/// 狀態條上一格點下去做什麼。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusAction {
    /// 「中 / 英」：切模式。
    ToggleMode,

    /// 「，。/ ,.」：切全形標點。
    TogglePunctuation,

    /// 齒輪：開啟設定程式（UI 執行緒直接起程序，不經 Router）。
    OpenSettings,
}
