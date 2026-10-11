use super::{Usage, UsageSummary};

/// 輸入統計的累計方。實現放學習 crate，Core 只往裡送每次上屏的用量。
pub trait UsageMeter: Send {
    /// 記一次上屏。實現不能阻塞輸入：只加記憶體裡的計數，落盤放 [`Self::flush`]。
    fn record(&mut self, usage: Usage);

    /// 落盤。殼在停用輸入法時呼叫，啟用期間也定時調；失敗只記日誌。
    fn flush(&mut self) {}

    /// 給偏好設定看的彙總。
    fn summary(&self) -> UsageSummary {
        UsageSummary::default()
    }
}

/// 不統計。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoUsageMeter;

impl UsageMeter for NoUsageMeter {
    fn record(&mut self, _usage: Usage) {}
}
