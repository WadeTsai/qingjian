//! 一段文字怎麼畫：字號、行高（畫素）、字重、斜體、字族、顏色、描邊、刪除線。

use crate::color::Color;
use crate::theme::{FamilyId, FontSpec, FontWeight};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextStyle {
    /// 字號（畫素）。
    pub size: f32,

    /// 字號（點）：光學字號與字距表按點查。
    pub points: f32,

    /// 行高（畫素）。
    pub line_height: f32,

    pub weight: FontWeight,

    /// 斜體；字型沒有斜體面時傾斜正體代替。
    pub italic: bool,

    /// 字族回退鏈的序號，查當前主題的字族表。
    pub family: FamilyId,

    pub color: Color,

    /// 描邊：向外的寬度（畫素）與顏色；描邊墊在字形底下，不佔排版寬度。
    pub stroke: Option<(f32, Color)>,

    /// 畫刪除線（糾錯改掉的拼音）。
    pub strike: bool,

    /// 畫下劃線（輔碼碼段）。
    pub underline: bool,

    /// 覆蓋率 gamma，見 `Theme::text_gamma`。
    pub gamma: f32,
}

impl TextStyle {
    /// `font` 已按倍數換成畫素；`points` 是換算前的字號。
    pub(crate) fn new(font: FontSpec, points: f32, color: Color, gamma: f32) -> Self {
        Self {
            size: font.size,
            points,
            line_height: font.line_height,
            weight: font.weight,
            italic: font.italic,
            family: font.family,
            color,
            stroke: None,
            strike: false,
            underline: false,
            gamma,
        }
    }

    pub(crate) fn stroked(mut self, stroke: Option<(f32, Color)>) -> Self {
        self.stroke = stroke.filter(|(width, _)| *width > 0.0);
        self
    }

    pub(crate) fn struck(mut self) -> Self {
        self.strike = true;
        self
    }

    pub(crate) fn underlined(mut self) -> Self {
        self.underline = true;
        self
    }
}
