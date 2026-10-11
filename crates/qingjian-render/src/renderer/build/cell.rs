//! `repeat` 展開狀態條格子時每一份繫結的那一格。

use crate::renderer::StatusCell;

#[derive(Debug, Clone, Copy)]
pub(super) struct CellContext<'a> {
    pub(super) cell: &'a StatusCell,

    /// 第幾格，從 0 起。
    pub(super) index: usize,

    /// 一共幾格。
    pub(super) count: usize,
}
