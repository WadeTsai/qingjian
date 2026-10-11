//! 主題裡顏色的幾種寫法：顏色引用（`#rrggbb[aa]` 或 `@名字`）、節點上按條件二選一、`variables` 裡的顏色值。

mod reference;
mod spec;
mod value;

pub(crate) use reference::{ColorRef, parse_hex};
pub(crate) use spec::ColorSpec;
pub(crate) use value::ColorValue;
