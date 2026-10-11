use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 詞表裡的一條：詞在 arena 裡的位置與一元計數。下標就是詞的編號。12 位元組、無填充，原樣落盤。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct WordEntry {
    /// 在 `BigramModel::words` 裡的位元組偏移。
    pub text_start: u32,

    /// 一元計數。
    pub count: u32,

    /// 位元組長度。
    pub text_len: u16,

    /// 對齊用，全零。
    pub reserved: u16,
}
