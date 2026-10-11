//! 一段文字量出來的尺寸（畫素）。

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextSize {
    pub width: f32,

    /// 等於所用樣式的行高。
    pub height: f32,

    /// 第一行的基線離行框頂邊多遠，與畫字時一樣取整。
    pub baseline: f32,
}
