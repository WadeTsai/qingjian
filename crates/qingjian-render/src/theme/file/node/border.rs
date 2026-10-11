//! 寬度 + 顏色：框的邊框（畫在盒子內側），也是文字的描邊（向外，墊在字形底下）；都不佔佈局空間。

use serde::Deserialize;

use crate::theme::file::ColorSpec;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct BorderSpec {
    /// 寬度（點）。
    pub(crate) width: f32,

    pub(crate) color: ColorSpec,
}
