//! `repeat` 展開時每一份繫結的那一項候選。

use crate::frame::Row;

#[derive(Debug, Clone, Copy)]
pub(super) struct RowContext<'a> {
    pub(super) row: &'a Row,

    /// 在這一頁裡的位置，從 0 起。
    pub(super) index: usize,

    /// 這一頁的候選數。
    pub(super) count: usize,
}
