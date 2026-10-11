use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 分節表裡的一條：標籤、正文在檔案裡的偏移與長度。24 位元組。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct SectionEntry {
    /// 4 位元組 ASCII 標籤（`META`、`TEXT`……）。
    pub tag: [u8; 4],

    /// 對齊用，全零。
    pub reserved: u32,

    /// 正文起始的檔案偏移，8 位元組對齊。
    pub offset: u64,

    /// 正文長度（不含對齊填充）。
    pub length: u64,
}

impl SectionEntry {
    pub const SIZE: usize = size_of::<Self>();
}
