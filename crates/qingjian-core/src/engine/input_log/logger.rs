use super::InputLogEntry;

/// 輸入日誌的落盤方。實現放學習 crate，Core 只往裡送條目。
pub trait InputLogger: Send {
    /// 記一條。實現不能阻塞輸入：寫失敗只記日誌。
    fn record(&mut self, entry: InputLogEntry);

    /// 把緩衝寫出去。殼在停用輸入法時呼叫。
    fn flush(&mut self) {}

    /// 有沒有在記（預設實現沒有，殼據此在診斷資訊裡說明）。
    fn is_enabled(&self) -> bool {
        true
    }
}

/// 不記。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoInputLogger;

impl InputLogger for NoInputLogger {
    fn record(&mut self, _entry: InputLogEntry) {}

    fn is_enabled(&self) -> bool {
        false
    }
}
