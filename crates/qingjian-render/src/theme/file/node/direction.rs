//! 框裡子節點的排列方向。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Direction {
    /// 橫向。
    #[default]
    Row,

    /// 縱向。
    Column,
}
