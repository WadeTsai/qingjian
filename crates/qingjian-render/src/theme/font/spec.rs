//! 一種字型用法：字號、行高（點）、字重、斜體與字族。字族是主題字族表裡的一條回退鏈，渲染器按裝了哪些字型挑出實際用的。

use super::{FamilyId, FontWeight};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontSpec {
    /// 字號。
    pub size: f32,

    /// 行高：一行文字佔的高度，字形在其中垂直居中。
    pub line_height: f32,

    pub weight: FontWeight,

    pub italic: bool,

    /// 字族回退鏈在主題字族表裡的序號。
    pub(crate) family: FamilyId,
}

impl FontSpec {
    /// 預設字族、常規字重的正體。
    pub const fn new(size: f32, line_height: f32) -> Self {
        Self {
            size,
            line_height,
            weight: FontWeight::REGULAR,
            italic: false,
            family: FamilyId(0),
        }
    }

    /// 點 → 畫素。
    pub(crate) fn scaled(self, scale: f32) -> Self {
        Self {
            size: self.size * scale,
            line_height: self.line_height * scale,
            ..self
        }
    }
}
