use std::ops::Deref;
use std::sync::Arc;

use memmap2::Mmap;

/// 一段 UTF-8 文本（arena）：從 TSV 解析出來時是自己的 `String`，從 `.qj` 開啟時是對映檔案裡的一段，
/// 開啟時校驗過一次 UTF-8。對外都是 `&str`。
#[derive(Debug)]
pub enum Text {
    /// 記憶體裡自己的。
    Owned(String),

    /// 對映檔案裡 `offset..offset + len` 的位元組。
    Mapped {
        /// 整個檔案的對映，多個分節共享。
        map: Arc<Mmap>,

        /// 正文起始偏移。
        offset: usize,

        /// 正文位元組數。
        len: usize,
    },
}

impl Default for Text {
    fn default() -> Self {
        Self::Owned(String::new())
    }
}

impl Text {
    /// 從對映裡取一段並校驗 UTF-8。
    pub(crate) fn mapped(map: Arc<Mmap>, offset: usize, len: usize) -> Option<Self> {
        let bytes = map.get(offset..offset + len)?;
        std::str::from_utf8(bytes).ok()?;
        Some(Self::Mapped { map, offset, len })
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            Self::Owned(text) => text,
            Self::Mapped { map, offset, len } => {
                let bytes = &map[*offset..*offset + *len];
                // SAFETY: `Text::mapped` 構造時對這段位元組做過 UTF-8 校驗；資料檔案只整體替換（寫臨時檔案再改名）、
                // 從不就地修改，對映內容在生命週期內不變。這裡不能再用帶校驗的版本：每次取 `&str` 都掃一遍幾十 MB 的 arena。
                unsafe { std::str::from_utf8_unchecked(bytes) }
            }
        }
    }
}
