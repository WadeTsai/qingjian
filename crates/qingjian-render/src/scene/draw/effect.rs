//! 畫投影與內陰影：把節點的形狀畫進一張離屏圖取 alpha，偏移、模糊、染色後合成。
//! 離屏圖只取形狀加模糊鋪開的那一塊，不是整張畫布。形狀怎麼畫由呼叫方給（節點自己的畫面，或容器的子節點）。
//! 算好的遮罩交回呼叫方合成並快取：動畫幀裡沒動的節點直接貼快取，不再重畫形狀、重做模糊。

use crate::canvas::Canvas;
use crate::error::RenderError;
use crate::scene::{Effect, EffectKind};
use crate::shadow;

/// 在離屏圖上畫形狀：盒子在 `rect`（離屏圖座標），按 `spread` 外擴（負數內縮）。
pub(in crate::scene) type DrawShape<'a> =
    dyn FnMut(&mut Canvas, (f32, f32, f32, f32), f32) -> Result<(), RenderError> + 'a;

/// 一個效果算好的遮罩：左上角在畫布裡的位置、寬高、alpha。
#[derive(Debug, Clone)]
pub(in crate::scene) struct EffectMask {
    pub(in crate::scene) left: i32,

    pub(in crate::scene) top: i32,

    pub(in crate::scene) width: u32,

    pub(in crate::scene) height: u32,

    pub(in crate::scene) alpha: Vec<u8>,
}

/// 算一個效果的遮罩；`rect` 是節點盒子（畫素，畫布座標），`bounds` 是畫布寬高。整塊落在畫布外時為 `None`。
pub(in crate::scene) fn effect_mask(
    bounds: (u32, u32),
    rect: (f32, f32, f32, f32),
    effect: &Effect,
    shape: &mut DrawShape,
) -> Result<Option<EffectMask>, RenderError> {
    let (x, y, width, height) = rect;
    let pad = shadow::reach(effect.blur) + effect.spread.abs();
    let (sx, sy) = match effect.kind {
        EffectKind::DropShadow => (x + effect.x, y + effect.y),
        EffectKind::InnerShadow => (x, y),
    };
    // 離屏區域：形狀（投影按偏移挪過）四周加模糊鋪開的寬度，夾在畫布裡
    let left = ((sx - pad).floor() as i32).max(0);
    let top = ((sy - pad).floor() as i32).max(0);
    let right = ((sx + width + pad).ceil() as i32).min(bounds.0 as i32);
    let bottom = ((sy + height + pad).ceil() as i32).min(bounds.1 as i32);
    if right <= left || bottom <= top {
        return Ok(None);
    }
    let (w, h) = ((right - left) as u32, (bottom - top) as u32);
    let (ox, oy) = (left as f32, top as f32);
    let shifted = (x + effect.x - ox, y + effect.y - oy, width, height);
    let mask = match effect.kind {
        EffectKind::DropShadow => {
            let mut mask = alpha(shape, shifted, effect.spread, (w, h))?;
            shadow::blur(&mut mask, w as usize, h as usize, effect.blur);
            mask
        }
        EffectKind::InnerShadow => {
            let inside = alpha(shape, (x - ox, y - oy, width, height), 0.0, (w, h))?;
            // 形狀外（按偏移挪過、按擴充套件縮過）為實，模糊後只留形狀裡的部分
            let mut mask = alpha(shape, shifted, -effect.spread, (w, h))?;
            for value in &mut mask {
                *value = 255 - *value;
            }
            shadow::blur(&mut mask, w as usize, h as usize, effect.blur);
            for (value, inside) in mask.iter_mut().zip(&inside) {
                *value = ((u32::from(*value) * u32::from(*inside) + 127) / 255) as u8;
            }
            mask
        }
    };
    Ok(Some(EffectMask {
        left,
        top,
        width: w,
        height: h,
        alpha: mask,
    }))
}

/// 形狀畫進一張 `size` 大的離屏圖，取 alpha。
fn alpha(
    shape: &mut DrawShape,
    rect: (f32, f32, f32, f32),
    spread: f32,
    size: (u32, u32),
) -> Result<Vec<u8>, RenderError> {
    let mut layer = Canvas::new(size.0, size.1)?;
    shape(&mut layer, rect, spread)?;
    Ok(layer
        .into_pixmap()
        .pixels()
        .iter()
        .map(|pixel| pixel.alpha())
        .collect())
}
