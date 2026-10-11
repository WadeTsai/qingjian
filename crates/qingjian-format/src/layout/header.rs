use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 檔案頭魔數。
pub const MAGIC: [u8; 8] = *b"QINGJIAN";

/// 當前格式版本。佈局不相容時加一，讀舊版本的程式碼按需保留。
pub const FORMAT_VERSION: u16 = 1;

/// 檔案頭，32 位元組，緊跟著 `section_count` 條 [`crate::SectionEntry`]。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct Header {
    /// [`MAGIC`]。
    pub magic: [u8; 8],

    /// [`FORMAT_VERSION`]。
    pub version: u16,

    /// 資料種類，見 [`crate::Kind`]。
    pub kind: u16,

    /// 分節數。
    pub section_count: u32,

    /// 留給以後（校驗和、標誌位），現在全零。
    pub reserved: [u8; 16],
}

impl Header {
    pub const SIZE: usize = size_of::<Self>();
}
