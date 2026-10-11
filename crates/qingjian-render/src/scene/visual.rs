//! 場景圖節點畫什麼：節點的盒子由佈局算出，這裡只描述在盒子裡怎麼畫。

use super::box_paint::BoxPaint;
use super::fill::Fill;
use super::icon::Icon;
use crate::color::Color;
use crate::text::TextStyle;

#[derive(Debug, Clone)]
pub(crate) enum Visual {
    /// 純容器，自己不畫。
    Group,

    /// 盒子：填充、邊框、圓角。
    Box(BoxPaint),

    /// 單行文字，盒子左上角是行框頂邊；佈局時按文字量寬高。
    Text { text: String, style: TextStyle },

    /// 圖示：邊長 `size`，在盒子裡垂直居中、靠左。
    Icon { icon: Icon, size: f32, color: Color },
}

impl Visual {
    /// 純色（圓角）矩形；`radius` 為 0 時畫直角。
    pub(crate) fn solid(color: Color, radius: f32) -> Self {
        Self::Box(BoxPaint {
            fill: Some(Fill::Solid(color)),
            border: None,
            radius,
        })
    }
}
