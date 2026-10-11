//! 一個盒子怎麼畫：填充、內側邊框、圓角（畫素）。

use super::fill::Fill;
use crate::color::Color;

#[derive(Debug, Clone)]
pub(crate) struct BoxPaint {
    pub(crate) fill: Option<Fill>,

    /// 內側邊框：寬度（畫素）與顏色。
    pub(crate) border: Option<(f32, Color)>,

    pub(crate) radius: f32,
}
