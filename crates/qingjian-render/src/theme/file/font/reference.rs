//! 節點裡的 `font`：`text.styles` 裡的樣式名，或在某個樣式上改幾項：
//! `{ "base": "candidate", "size": 18, "weight": "semibold", "style": "italic" }`。只改字號不寫行高時，行高按字號等比縮放。
//! 字族不能在這裡改：換字族要定義一個命名樣式。

use serde::Deserialize;

use super::FontStyle;
use crate::theme::{FontSpec, FontWeight};

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum FontRef {
    Named(String),

    Inline {
        /// 在哪個樣式上改；不寫從預設樣式改。
        base: Option<String>,

        size: Option<f32>,

        line_height: Option<f32>,

        weight: Option<FontWeight>,

        style: Option<FontStyle>,
    },
}

impl FontRef {
    /// 引用的樣式名。
    pub(crate) fn base(&self) -> Option<&str> {
        match self {
            Self::Named(name) => Some(name),
            Self::Inline { base, .. } => base.as_deref(),
        }
    }

    /// 在基礎樣式上套用這裡的改動。
    pub(crate) fn apply(&self, base: FontSpec) -> FontSpec {
        let Self::Inline {
            size,
            line_height,
            weight,
            style,
            ..
        } = self
        else {
            return base;
        };
        let size = size.unwrap_or(base.size);
        FontSpec {
            size,
            line_height: line_height.unwrap_or(if base.size > 0.0 {
                base.line_height * size / base.size
            } else {
                size
            }),
            weight: weight.unwrap_or(base.weight),
            italic: style.map_or(base.italic, |style| style == FontStyle::Italic),
            family: base.family,
        }
    }
}
