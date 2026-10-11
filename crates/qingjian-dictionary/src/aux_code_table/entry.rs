//! 碼錶的落盤條目：一個詞與它的一個碼。

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 碼錶裡的一條：一個詞與它的一個碼。12 位元組、無填充，原樣落盤。
///
/// 條目按詞文本的位元組序升序排列，同一個詞的條目相鄰；`HASH` 索引指向這段區間的起點。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(super) struct CodeEntry {
    /// 詞文本在詞 arena 裡的位元組偏移。
    pub word_start: u32,

    /// 碼在碼 arena 裡的位元組偏移。
    pub code_start: u32,

    /// 詞文本的位元組長度。
    pub word_len: u16,

    /// 碼的位元組長度（1–8）。
    pub code_len: u8,

    /// 對齊用，全零。
    pub reserved: u8,
}
