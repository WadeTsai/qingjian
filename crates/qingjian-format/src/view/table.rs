use std::ops::Deref;
use std::sync::Arc;

use memmap2::Mmap;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 一段定長結構體陣列：從 TSV 解析出來時是自己的 `Vec`，從 `.qj` 開啟時是對映檔案裡的一段。
/// 兩者對外都是 `&[T]`（`Deref`），查詢程式碼不區分。
#[derive(Debug)]
pub enum Table<T> {
    /// 記憶體裡自己的。
    Owned(Vec<T>),

    /// 對映檔案裡 `offset..offset + len` 的位元組，開啟時已校驗對齊與長度。
    Mapped {
        /// 整個檔案的對映，多個分節共享。
        map: Arc<Mmap>,

        /// 正文起始偏移。
        offset: usize,

        /// 正文位元組數。
        len: usize,
    },
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Self::Owned(Vec::new())
    }
}

impl<T: FromBytes + Immutable + KnownLayout> Table<T> {
    /// 從對映裡取一段，校驗對齊與整除。
    pub(crate) fn mapped(map: Arc<Mmap>, offset: usize, len: usize) -> Option<Self> {
        let bytes = map.get(offset..offset + len)?;
        <[T]>::ref_from_bytes(bytes).ok()?;
        Some(Self::Mapped { map, offset, len })
    }
}

impl<T: FromBytes + Immutable + KnownLayout> Deref for Table<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        match self {
            Self::Owned(items) => items,
            Self::Mapped { map, offset, len } => {
                <[T]>::ref_from_bytes(&map[*offset..*offset + *len])
                    .expect("mapped table was validated when the container was opened")
            }
        }
    }
}

impl<T: FromBytes + IntoBytes + Immutable + KnownLayout> Table<T> {
    /// 落盤用的位元組形式。
    pub fn as_bytes(&self) -> &[u8] {
        let items: &[T] = self;
        items.as_bytes()
    }
}
