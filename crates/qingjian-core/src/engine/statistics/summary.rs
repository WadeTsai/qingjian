use super::Usage;

/// 輸入統計的彙總，偏好設定「統計」頁顯示的就是它。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageSummary {
    /// 今天。
    pub today: Usage,

    /// 最近 7 天（含今天）。
    pub week: Usage,

    /// 從有記錄起的累計。
    pub total: Usage,

    /// 有過上屏的天數。
    pub days: u32,

    /// 最早一條記錄的日期（`YYYY-MM-DD`）；沒記錄為 `None`。
    pub since: Option<String>,
}
