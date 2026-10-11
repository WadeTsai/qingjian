//! 雲聯想的小云朵：兩個圓拱加一條圓角底邊的並集，只描邊不填充，對應 macOS 的 SF Symbol `cloud`（18×13 擠進 13×13 方塊）
//! 與 Windows 的 ☁ 字形。

use tiny_skia::{BlendMode, PathBuilder, Pixmap, Rect};

use crate::canvas::Canvas;
use crate::color::Color;

/// 描邊寬度相對邊長的比例。
const STROKE_RATIO: f32 = 0.08;

/// 畫在 `(x, y)` 為左上角、邊長 `size` 畫素的方塊裡。
pub(crate) fn draw_cloud(canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
    let side = size.ceil() as u32 + 1;
    let Some(icon) = Pixmap::new(side, side) else {
        return;
    };
    let stroke = (size * STROKE_RATIO).max(1.0);
    let mut layer = Canvas::from_pixmap(icon);
    if let Some(outer) = cloud_shape(size, 0.0) {
        layer.fill_path(&outer, color, BlendMode::SourceOver);
    }
    // 挖掉向內收一圈的同形，剩下的就是描邊
    if let Some(inner) = cloud_shape(size, stroke) {
        layer.fill_path(&inner, Color::rgb(0, 0, 0), BlendMode::Clear);
    }
    let icon = layer.into_pixmap();
    canvas.blend_pixmap(x.round() as i32, y.round() as i32, &icon);
}

/// 雲朵輪廓：向內收 `inset` 畫素的版本用來挖空。座標按邊長歸一化。
fn cloud_shape(size: f32, inset: f32) -> Option<tiny_skia::Path> {
    let s = size;
    let mut path = PathBuilder::new();
    let mut circle = |cx: f32, cy: f32, r: f32| {
        let radius = r * s - inset;
        if radius > 0.0 {
            path.push_circle(cx * s, cy * s, radius);
        }
    };
    // 左邊小拱、右邊大拱、底邊兩個圓角
    circle(0.34, 0.60, 0.23);
    circle(0.63, 0.50, 0.30);
    circle(0.25, 0.72, 0.16);
    circle(0.77, 0.72, 0.16);
    // 底邊：把兩個圓角之間填平
    let (top, bottom) = (0.60 * s, 0.88 * s - inset);
    if bottom > top
        && let Some(rect) = Rect::from_ltrb(0.25 * s, top, 0.77 * s, bottom)
    {
        path.push_rect(rect);
    }
    path.finish()
}
