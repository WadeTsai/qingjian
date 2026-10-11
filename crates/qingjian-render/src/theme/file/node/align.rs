//! 節點在父框交叉軸上的對齊。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Align {
    Start,

    End,

    Center,

    /// 撐滿。
    Stretch,
}
