//! 候選視窗兩種排布各自的根節點。

use serde::Deserialize;

use super::node::NodeSpec;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Windows {
    /// 豎排：一行一個候選。
    pub(crate) vertical: NodeSpec,

    /// 橫排：候選排成一行。
    pub(crate) horizontal: NodeSpec,
}
