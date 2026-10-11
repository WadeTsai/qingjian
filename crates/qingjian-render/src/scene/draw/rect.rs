//! 畫一個盒子：填充（純色 / 漸變 / 圖片 / SVG）再畫內側邊框。

use tiny_skia::{
    FilterQuality, GradientStop, LinearGradient, Mask, Pattern, Point, RadialGradient, Rect,
    Shader, SpreadMode, Transform,
};

use crate::canvas::{Canvas, round_rect};
use crate::color::Color;
use crate::scene::{BoxPaint, Fill};

/// 在 `(x, y, width, height)`（畫素）畫 `paint`。
pub(in crate::scene) fn draw_box(
    canvas: &mut Canvas,
    rect: (f32, f32, f32, f32),
    paint: &BoxPaint,
) {
    let (x, y, width, height) = rect;
    let radius = paint.radius;
    match &paint.fill {
        Some(Fill::Solid(color)) if radius > 0.0 => {
            canvas.fill_round_rect(x, y, width, height, radius, *color);
        }
        Some(Fill::Solid(color)) => canvas.fill_rect(x, y, width, height, *color),
        Some(Fill::Linear { angle, stops }) => {
            if let (Some(path), Some(shader)) = (
                round_rect(x, y, width, height, radius),
                linear(rect, *angle, stops),
            ) {
                canvas.fill_path_with(&path, shader, None);
            }
        }
        Some(Fill::Radial { center, stops }) => {
            if let (Some(path), Some(shader)) = (
                round_rect(x, y, width, height, radius),
                radial(rect, *center, stops),
            ) {
                canvas.fill_path_with(&path, shader, None);
            }
        }
        Some(Fill::Image {
            pixmap,
            slice,
            pixels_per_px,
        }) => {
            let mask = corner_mask(canvas, rect, radius);
            match slice {
                Some(slice) => {
                    nine_slice(canvas, rect, pixmap, *slice, *pixels_per_px, mask.as_ref())
                }
                None => stretch(canvas, rect, pixmap, mask.as_ref()),
            }
        }
        Some(Fill::Svg {
            image,
            slice,
            px_per_unit,
        }) => {
            let mask = corner_mask(canvas, rect, radius);
            match slice {
                // 九宮格按原始尺寸柵格，四角按倍數原樣貼；切邊從 SVG 單位換成柵格畫素
                Some(slice) => {
                    let (svg_width, svg_height) = image.size();
                    let raster = image.raster(
                        (svg_width * px_per_unit).round() as u32,
                        (svg_height * px_per_unit).round() as u32,
                    );
                    if let Some(pixmap) = raster {
                        let per_unit = pixmap.width() as f32 / svg_width;
                        let slice = slice.map(|edge| edge * per_unit);
                        nine_slice(canvas, rect, &pixmap, slice, 1.0, mask.as_ref());
                    }
                }
                // 拉伸直接按盒子的畫素尺寸柵格，不經過點陣圖縮放
                None => {
                    if let Some(pixmap) = image.raster(width.round() as u32, height.round() as u32)
                    {
                        stretch(canvas, rect, &pixmap, mask.as_ref());
                    }
                }
            }
        }
        None => {}
    }
    if let Some((line, color)) = paint.border
        && line > 0.0
        && let Some(path) = round_rect(
            x + line / 2.0,
            y + line / 2.0,
            width - line,
            height - line,
            (radius - line / 2.0).max(0.0),
        )
    {
        canvas.stroke_path(&path, line, color);
    }
}

fn gradient_stops(stops: &[(Color, f32)]) -> Vec<GradientStop> {
    stops
        .iter()
        .map(|(color, position)| GradientStop::new(*position, color.to_skia()))
        .collect()
}

/// CSS 式線性漸變：漸變線過盒子中心，長度讓兩端正好碰到對角。
fn linear(
    rect: (f32, f32, f32, f32),
    angle: f32,
    stops: &[(Color, f32)],
) -> Option<Shader<'static>> {
    let (x, y, width, height) = rect;
    let radians = angle.to_radians();
    let (dx, dy) = (radians.sin(), -radians.cos());
    let half = (width * dx.abs() + height * dy.abs()) / 2.0;
    let (cx, cy) = (x + width / 2.0, y + height / 2.0);
    LinearGradient::new(
        Point::from_xy(cx - dx * half, cy - dy * half),
        Point::from_xy(cx + dx * half, cy + dy * half),
        gradient_stops(stops),
        SpreadMode::Pad,
        Transform::identity(),
    )
}

/// 徑向漸變：圓心按比例，半徑到最遠的角。
fn radial(
    rect: (f32, f32, f32, f32),
    center: [f32; 2],
    stops: &[(Color, f32)],
) -> Option<Shader<'static>> {
    let (x, y, width, height) = rect;
    let (cx, cy) = (x + width * center[0], y + height * center[1]);
    let radius = [
        (x, y),
        (x + width, y),
        (x, y + height),
        (x + width, y + height),
    ]
    .into_iter()
    .map(|(px, py)| ((px - cx).powi(2) + (py - cy).powi(2)).sqrt())
    .fold(0.0_f32, f32::max);
    let center = Point::from_xy(cx, cy);
    RadialGradient::new(
        center,
        0.0,
        center,
        radius,
        gradient_stops(stops),
        SpreadMode::Pad,
        Transform::identity(),
    )
}

/// 圓角框裁圖片用的遮罩；沒有圓角為 `None`。
fn corner_mask(canvas: &Canvas, rect: (f32, f32, f32, f32), radius: f32) -> Option<Mask> {
    let (x, y, width, height) = rect;
    (radius > 0.0)
        .then(|| round_rect(x, y, width, height, radius))
        .flatten()
        .and_then(|path| canvas.mask(&path))
}

/// 整張圖拉伸到盒子。
fn stretch(
    canvas: &mut Canvas,
    rect: (f32, f32, f32, f32),
    pixmap: &tiny_skia::Pixmap,
    mask: Option<&Mask>,
) {
    let (x, y, width, height) = rect;
    let (image_width, image_height) = (pixmap.width() as f32, pixmap.height() as f32);
    let transform = Transform::from_row(width / image_width, 0.0, 0.0, height / image_height, x, y);
    fill_cell(canvas, (x, y, width, height), pixmap, transform, mask);
}

/// 九宮格：四角原樣（按倍數縮放），四邊沿一個方向拉伸，中間兩個方向拉伸。
fn nine_slice(
    canvas: &mut Canvas,
    rect: (f32, f32, f32, f32),
    pixmap: &tiny_skia::Pixmap,
    slice: [f32; 4],
    pixels_per_px: f32,
    mask: Option<&Mask>,
) {
    let (x, y, width, height) = rect;
    let (image_width, image_height) = (pixmap.width() as f32, pixmap.height() as f32);
    let [top, right, bottom, left] = slice;
    let per = pixels_per_px.max(f32::EPSILON);
    // 源（圖片畫素）與目標（畫布畫素）的四條分割線；目標四角按倍數，放不下時壓縮到盒子裡。
    // 內部兩條目標分割線取整畫素：相鄰兩塊各自抗鋸齒會在接縫留一道 alpha 不滿的縫，底下有投影就透出來
    let source_x = [0.0, left, image_width - right, image_width];
    let source_y = [0.0, top, image_height - bottom, image_height];
    let shrink_x = ((left + right) / per / width).max(1.0);
    let shrink_y = ((top + bottom) / per / height).max(1.0);
    let target_x = [
        x,
        (x + left / per / shrink_x).round(),
        (x + width - right / per / shrink_x).round(),
        x + width,
    ];
    let target_y = [
        y,
        (y + top / per / shrink_y).round(),
        (y + height - bottom / per / shrink_y).round(),
        y + height,
    ];
    for row in 0..3 {
        for column in 0..3 {
            let (sx0, sx1) = (source_x[column], source_x[column + 1]);
            let (sy0, sy1) = (source_y[row], source_y[row + 1]);
            let (tx0, tx1) = (target_x[column], target_x[column + 1]);
            let (ty0, ty1) = (target_y[row], target_y[row + 1]);
            if sx1 <= sx0 || sy1 <= sy0 || tx1 <= tx0 || ty1 <= ty0 {
                continue;
            }
            // 這一格在原圖裡是單一顏色（Skia 畫九宮格時的「固定顏色格」）：純色填，不做圖片取樣
            if let Some(color) = uniform_color(pixmap, (sx0, sy0, sx1, sy1)) {
                if let Some(rect) = Rect::from_xywh(tx0, ty0, tx1 - tx0, ty1 - ty0) {
                    let path = tiny_skia::PathBuilder::from_rect(rect);
                    let paint = tiny_skia::Paint {
                        shader: Shader::SolidColor(color),
                        anti_alias: true,
                        ..tiny_skia::Paint::default()
                    };
                    canvas.fill_path_paint(&path, &paint, mask);
                }
                continue;
            }
            let scale_x = (tx1 - tx0) / (sx1 - sx0);
            let scale_y = (ty1 - ty0) / (sy1 - sy0);
            let transform = Transform::from_row(
                scale_x,
                0.0,
                0.0,
                scale_y,
                tx0 - sx0 * scale_x,
                ty0 - sy0 * scale_y,
            );
            fill_cell(
                canvas,
                (tx0, ty0, tx1 - tx0, ty1 - ty0),
                pixmap,
                transform,
                mask,
            );
        }
    }
}

/// 原圖 `(x0, y0, x1, y1)` 這塊（圖片畫素，取整到整畫素）所有畫素同色時返回那個顏色。
fn uniform_color(
    pixmap: &tiny_skia::Pixmap,
    (x0, y0, x1, y1): (f32, f32, f32, f32),
) -> Option<tiny_skia::Color> {
    let (x0, y0) = (x0.floor() as u32, y0.floor() as u32);
    let (x1, y1) = (
        (x1.ceil() as u32).min(pixmap.width()),
        (y1.ceil() as u32).min(pixmap.height()),
    );
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let pixels = pixmap.pixels();
    let width = pixmap.width() as usize;
    let first = pixels[y0 as usize * width + x0 as usize];
    let same = (y0..y1).all(|y| {
        pixels[y as usize * width + x0 as usize..y as usize * width + x1 as usize]
            .iter()
            .all(|pixel| *pixel == first)
    });
    let color = first.demultiply();
    same.then(|| {
        tiny_skia::Color::from_rgba8(color.red(), color.green(), color.blue(), color.alpha())
    })
}

/// 用圖片圖案填一塊矩形。
fn fill_cell(
    canvas: &mut Canvas,
    cell: (f32, f32, f32, f32),
    pixmap: &tiny_skia::Pixmap,
    transform: Transform,
    mask: Option<&Mask>,
) {
    let (x, y, width, height) = cell;
    let Some(rect) = Rect::from_xywh(x, y, width, height) else {
        return;
    };
    let shader = Pattern::new(
        pixmap.as_ref(),
        SpreadMode::Pad,
        FilterQuality::Bilinear,
        1.0,
        transform,
    );
    canvas.fill_path_with(&tiny_skia::PathBuilder::from_rect(rect), shader, mask);
}
