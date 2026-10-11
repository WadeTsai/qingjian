/// 使用者在狀態條上做的事，UI 執行緒發回 Router（開啟設定不經過 Router，UI 執行緒自己起程序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusEvent {
    /// 點了「中 / 英」格：翻轉模式。
    ToggleMode,

    /// 點了「，。」格：翻轉當前模式的全形標點（中英各記一份）。
    TogglePunctuation,

    /// 拖動結束，內容左上角的新位置（物理畫素）。
    Moved(i32, i32),
}
