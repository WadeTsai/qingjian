//! 拼音行裡的游標。

use serde::Deserialize;

use crate::theme::file::ColorSpec;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct CaretSpec {
    /// 寬度（點）。
    pub(crate) width: f32,

    pub(crate) color: ColorSpec,
}
