use std::time::Duration;

/// 連通性測試成功時的結果。
pub struct ConnectionReport {
    /// 請求用的模型名。
    pub model: String,

    /// 從發出到收到回覆的時間。
    pub elapsed: Duration,

    /// 模型的原始回覆（只用來在日誌裡對一眼）。
    pub reply: String,
}
