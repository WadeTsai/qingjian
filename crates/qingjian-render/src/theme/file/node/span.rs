//! 表格裡跨列：`row` 橫跨整行（高亮條）。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Span {
    Row,
}
