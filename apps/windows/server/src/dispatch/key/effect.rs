//! 一次按鍵對組句的影響。

/// 一次按鍵對組句的影響。
pub(crate) enum Effect {
    /// 緩衝變了，要重建 [`Composed`](super::super::Composed)；帶本次要上屏的文本。
    Changed(Option<String>),

    /// 只挪了高亮 / 翻頁。
    Navigated,

    /// 不吃，交還應用。
    Passthrough,
}
