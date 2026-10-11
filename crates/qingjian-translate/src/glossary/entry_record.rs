use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// `.qj` 裡的一條：詞在 arena 裡的位置，以及它的釋義在釋義表裡的一段。12 位元組、無填充。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct EntryRecord {
    /// 詞的位元組偏移。
    pub word_start: u32,

    /// 第一條釋義在 `SenseRecord` 表裡的下標。
    pub sense_start: u32,

    /// 詞的位元組長度。
    pub word_len: u16,

    /// 釋義條數。
    pub sense_count: u16,
}
