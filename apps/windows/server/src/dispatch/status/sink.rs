use super::StatusView;

/// 狀態條輸出端。Router 在工人執行緒上調，視窗在 UI 執行緒上，故要 `Send`。
pub trait StatusSink: Send {
    fn show_status(&self, view: StatusView);

    fn hide_status(&self);

    /// 起設定程式（工作列圖示右鍵選單用；懸浮條上的齒輪在 UI 執行緒直接起）。
    fn open_settings(&self) {}

    /// 用瀏覽器開啟下載頁（右鍵選單的「有新版本」）。
    fn open_download(&self) {}
}

/// 不畫狀態條的空實現。
pub struct NoopStatusSink;

impl StatusSink for NoopStatusSink {
    fn show_status(&self, _view: StatusView) {}

    fn hide_status(&self) {}
}
