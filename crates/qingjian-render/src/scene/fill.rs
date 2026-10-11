//! 盒子的填充：純色、線性 / 徑向漸變、圖片（PNG 或 SVG）。顏色已按外觀與資料解析好，長度已是畫素。

use std::sync::Arc;

use tiny_skia::Pixmap;

use crate::color::Color;
use crate::theme::SvgImage;

#[derive(Debug, Clone)]
pub(crate) enum Fill {
    Solid(Color),

    /// 角度同 CSS（度），色標（顏色、0–1 位置）。
    Linear {
        angle: f32,
        stops: Vec<(Color, f32)>,
    },

    /// 圓心按盒子寬高的比例，半徑到最遠的角。
    Radial {
        center: [f32; 2],

        stops: Vec<(Color, f32)>,
    },

    /// 圖片：`slice` 是九宮格切邊（圖片畫素，上右下左），`None` 整張拉伸；`pixels_per_px` 是一個畫布畫素對幾個圖片畫素。
    Image {
        pixmap: Arc<Pixmap>,

        slice: Option<[f32; 4]>,

        pixels_per_px: f32,
    },

    /// SVG：畫的時候按盒子（拉伸）或原始尺寸 × `px_per_unit`（九宮格）柵格；`slice` 是 SVG 單位的切邊。
    Svg {
        image: Arc<SvgImage>,

        slice: Option<[f32; 4]>,

        /// 一個 SVG 單位（點）對幾個畫布畫素。
        px_per_unit: f32,
    },
}
