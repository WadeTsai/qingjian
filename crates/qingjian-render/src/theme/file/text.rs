//! 文字設定：預設字族、命名的文字樣式（節點用 `"font": "名字"` 引用）與覆蓋率 gamma。

use std::collections::HashMap;

use serde::Deserialize;

use super::adaptive::Adaptive;
use super::{FamilyList, FontStyle};
use crate::theme::FontWeight;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TextSettings {
    /// 文字抗鋸齒覆蓋率的 gamma：小於 1 筆畫顯粗，見 `docs/design/rendering.md`。
    pub(crate) gamma: Adaptive<f32>,

    /// 沒寫字族的樣式用的字族；不寫為介面字型。
    #[serde(default)]
    pub(crate) family: Option<FamilyList>,

    /// 命名的文字樣式。
    pub(crate) styles: HashMap<String, StyleSpec>,
}

/// 一個命名文字樣式（點）。
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StyleSpec {
    pub(crate) size: f32,

    /// 行高：一行文字佔的高度，字形在其中垂直居中。
    pub(crate) line_height: f32,

    #[serde(default)]
    pub(crate) weight: FontWeight,

    /// 不寫用 `text.family`。
    #[serde(default)]
    pub(crate) family: Option<FamilyList>,

    #[serde(default)]
    pub(crate) style: FontStyle,
}
