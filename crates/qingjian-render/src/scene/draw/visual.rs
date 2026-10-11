//! 畫一個節點自己的畫面（不含子節點）；陰影取形狀時盒子可以外擴 / 內縮。

use super::draw_box;
use crate::canvas::Canvas;
use crate::cloud::draw_cloud;
use crate::gear::draw_gear;
use crate::scene::{BoxPaint, Icon, Visual};
use crate::text::TextPainter;

/// `rect` 是節點盒子（畫素，畫布座標）。
pub(in crate::scene) fn draw_visual(
    canvas: &mut Canvas,
    text: &mut TextPainter,
    visual: &Visual,
    rect: (f32, f32, f32, f32),
) {
    let (x, y, _, height) = rect;
    match visual {
        Visual::Box(paint) => draw_box(canvas, rect, paint),
        Visual::Text {
            text: content,
            style,
        } => {
            text.draw(canvas, content, style, x, y);
        }
        Visual::Icon { icon, size, color } => {
            let top = y + (height - size) / 2.0;
            match icon {
                Icon::Cloud => draw_cloud(canvas, x, top, *size, *color),
                Icon::Gear => draw_gear(canvas, x, top, *size, *color),
            }
        }
        Visual::Group => {}
    }
}

/// 陰影用的形狀：盒子按 `spread` 外擴（負數內縮），圓角隨之變；別的畫面不管 `spread`。
pub(in crate::scene) fn draw_shape(
    canvas: &mut Canvas,
    text: &mut TextPainter,
    visual: &Visual,
    rect: (f32, f32, f32, f32),
    spread: f32,
) {
    let Visual::Box(paint) = visual else {
        return draw_visual(canvas, text, visual, rect);
    };
    let (x, y, width, height) = rect;
    let grown = (
        x - spread,
        y - spread,
        (width + spread * 2.0).max(0.0),
        (height + spread * 2.0).max(0.0),
    );
    let paint = BoxPaint {
        radius: if paint.radius > 0.0 {
            (paint.radius + spread).max(0.0)
        } else {
            0.0
        },
        ..paint.clone()
    };
    draw_box(canvas, grown, &paint);
}
