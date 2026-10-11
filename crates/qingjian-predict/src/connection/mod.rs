//! 偏好設定「測試連線」：起執行緒發一條最小請求（`ConnectionTest`），結果是 `ConnectionReport`。

mod report;
mod test;

pub use report::ConnectionReport;
pub use test::ConnectionTest;
