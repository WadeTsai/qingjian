//! 尚未上屏的原樣文本及其 UTF-8 位元組游標。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawPreedit {
    /// 與相同狀態下 `Engine::take_raw()` 返回的完整文本一致。
    pub text: String,

    /// `text` 中的合法 UTF-8 字元邊界，空文本為 0。
    pub cursor_bytes: usize,
}
