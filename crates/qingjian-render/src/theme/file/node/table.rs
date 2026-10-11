//! 表格：`repeat` 出來的每個元件是一行，元件的子節點依次是各列；每列取各行最寬，行高固定。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct TableSpec {
    /// 行高（點）。
    pub(crate) row_height: f32,
}
