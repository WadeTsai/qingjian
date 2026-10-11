use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// `.qj` 裡的一條釋義：譯文、讀音、詞性縮寫都是 arena 裡的一段（長度 0 表示沒有）。20 位元組、無填充。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct SenseRecord {
    /// 譯文起點。
    pub text_start: u32,

    /// 讀音起點。
    pub reading_start: u32,

    /// 詞性縮寫起點。
    pub pos_start: u32,

    /// 譯文長度。
    pub text_len: u16,

    /// 讀音長度。
    pub reading_len: u16,

    /// 詞性縮寫長度。
    pub pos_len: u16,

    /// 對齊用，全零。
    pub reserved: u16,
}
