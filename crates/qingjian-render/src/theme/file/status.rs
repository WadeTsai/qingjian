//! 懸浮狀態條（Windows）的樣式。不寫 `root` 時格子的排法固定（每格內容居中、格間細線），用這裡的尺寸與顏色；
//! 寫了 `root` 就按節點樹畫（與候選視窗同一套寫法），格子用 `repeat` 繫結 `cells`。

use serde::Deserialize;

use super::ColorRef;
use super::FontRef;
use super::node::{EffectSpec, NodeSpec};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StatusSpec {
    /// 格內文字的樣式。
    pub(crate) font: FontRef,

    /// 每格左右的內邊距，也決定條的高度（行高 + 內邊距）。
    pub(crate) padding: f32,

    /// 圓角。
    pub(crate) radius: f32,

    pub(crate) background: ColorRef,

    /// 格間細線。
    pub(crate) separator: ColorRef,

    pub(crate) separator_width: f32,

    /// 普通文字。
    pub(crate) normal: ColorRef,

    /// 強調的文字（當前模式、生效中的全形標點）。
    pub(crate) emphasized: ColorRef,

    /// 齒輪。
    pub(crate) gear: ColorRef,

    pub(crate) gear_size: f32,

    /// 整條的投影、內陰影。
    #[serde(default)]
    pub(crate) effects: Vec<EffectSpec>,

    /// 節點樹畫法的根節點；寫了它上面幾項就不用了。
    #[serde(default)]
    pub(crate) root: Option<NodeSpec>,
}
