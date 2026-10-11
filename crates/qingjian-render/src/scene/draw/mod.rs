//! 畫單個節點：節點自己的畫面（盒子、文字、圖示）與它的投影、內陰影。按樹序遍歷在 `paint.rs`。

mod effect;
mod rect;
mod visual;

pub(super) use effect::{EffectMask, effect_mask};
pub(super) use rect::draw_box;
pub(super) use visual::{draw_shape, draw_visual};
