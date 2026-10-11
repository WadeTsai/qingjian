//! 雲服務頁「測試連線」的狀態，掛在 [`Settings`](super::Settings) 上，由根元件的 `update` 推進、雲服務頁顯示。

/// 「測試連線」的狀態。
#[derive(Clone)]
pub(crate) enum CloudStatus {
    /// 沒測過。
    Idle,

    /// 正在測。
    Testing,

    /// 成功（帶說明）。
    Ok(String),

    /// 失敗（帶錯誤）。
    Failed(String),
}
