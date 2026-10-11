//! 候選佈局的一格；空位不攜帶候選，由各平臺繪製和跳過。

use super::super::Candidate;

/// 排布里的一格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell<'a> {
    /// 本地候選。
    Local(&'a Candidate),

    /// 雲端詞。
    Cloud(&'a Candidate),

    /// 固定位置之前尚無真實候選的空格。
    Empty,
}

impl<'a> Cell<'a> {
    pub fn candidate(self) -> Option<&'a Candidate> {
        match self {
            Cell::Local(c) | Cell::Cloud(c) => Some(c),
            Cell::Empty => None,
        }
    }
}
