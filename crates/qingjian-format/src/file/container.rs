use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use memmap2::Mmap;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::{
    FORMAT_VERSION, FormatError, Header, Kind, MAGIC, META_TAG, Metadata, SECTION_ALIGN,
    SectionEntry, Table, Text,
};

/// 開啟的 `.qj` 檔案：整個檔案一次 mmap，分節按標籤取。
#[derive(Debug)]
pub struct Container {
    /// 檔案對映，交給取出去的 [`Table`] / [`Text`] 共享。
    map: Arc<Mmap>,

    /// 資料種類。
    kind: Kind,

    /// 標籤 → (偏移, 長度)。
    sections: HashMap<[u8; 4], (usize, usize)>,

    /// 第一節解析出來的後設資料。
    metadata: Metadata,
}

impl Container {
    /// 開啟並校驗：魔數、版本、種類、分節表邊界、`META` 可解析。
    pub fn open(path: &Path, expected: Kind) -> Result<Self, FormatError> {
        let file = File::open(path)?;
        // SAFETY: 資料檔案只整體替換（寫臨時檔案再改名），從不就地修改；對映期間內容不變。
        let map = unsafe { Mmap::map(&file)? };
        // `advise` 是 unix 專有（memmap2 裡 `Advice` gated 在 cfg(unix)）：Windows 上跳過，只少一個預讀提示。
        #[cfg(unix)]
        let _ = map.advise(memmap2::Advice::WillNeed);
        Self::from_map(Arc::new(map), expected)
    }

    /// 檔案開頭是不是 `.qj` 魔數（呼叫方據此在 TSV 與 `.qj` 之間選載入路徑）。
    pub fn is_qj(path: &Path) -> bool {
        use std::io::Read;
        let mut magic = [0u8; MAGIC.len()];
        File::open(path)
            .and_then(|mut f| f.read_exact(&mut magic))
            .is_ok()
            && magic == MAGIC
    }

    fn from_map(map: Arc<Mmap>, expected: Kind) -> Result<Self, FormatError> {
        let header = Header::ref_from_prefix(&map[..])
            .map(|(h, _)| *h)
            .map_err(|_| FormatError::BadMagic)?;
        if header.magic != MAGIC {
            return Err(FormatError::BadMagic);
        }
        if header.version != FORMAT_VERSION {
            return Err(FormatError::UnsupportedVersion(header.version));
        }
        let kind = Kind::from_code(header.kind).ok_or(FormatError::WrongKind {
            expected,
            found: header.kind,
        })?;
        if kind != expected {
            return Err(FormatError::WrongKind {
                expected,
                found: header.kind,
            });
        }
        let count = header.section_count as usize;
        let table_bytes = map
            .get(Header::SIZE..Header::SIZE + count * SectionEntry::SIZE)
            .ok_or(FormatError::Truncated)?;
        let entries =
            <[SectionEntry]>::ref_from_bytes(table_bytes).map_err(|_| FormatError::Truncated)?;
        let mut sections = HashMap::with_capacity(count);
        for entry in entries {
            let offset = usize::try_from(entry.offset).map_err(|_| FormatError::Truncated)?;
            let length = usize::try_from(entry.length).map_err(|_| FormatError::Truncated)?;
            if offset % SECTION_ALIGN != 0
                || offset.checked_add(length).is_none_or(|end| end > map.len())
            {
                return Err(FormatError::Truncated);
            }
            sections.insert(entry.tag, (offset, length));
        }
        let &(offset, length) = sections
            .get(&META_TAG)
            .ok_or(FormatError::MissingSection(META_TAG))?;
        let metadata = std::str::from_utf8(&map[offset..offset + length])
            .map_err(|_| FormatError::Malformed {
                tag: META_TAG,
                reason: "metadata is not UTF-8",
            })
            .and_then(|text| Metadata::from_toml(text).map_err(FormatError::from))?;
        Ok(Self {
            map,
            kind,
            sections,
            metadata,
        })
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// 原始位元組。
    pub fn bytes(&self, tag: [u8; 4]) -> Result<&[u8], FormatError> {
        let &(offset, length) = self
            .sections
            .get(&tag)
            .ok_or(FormatError::MissingSection(tag))?;
        Ok(&self.map[offset..offset + length])
    }

    /// 定長結構體陣列分節。
    pub fn table<T: FromBytes + Immutable + KnownLayout>(
        &self,
        tag: [u8; 4],
    ) -> Result<Table<T>, FormatError> {
        let &(offset, length) = self
            .sections
            .get(&tag)
            .ok_or(FormatError::MissingSection(tag))?;
        Table::mapped(Arc::clone(&self.map), offset, length).ok_or(FormatError::Malformed {
            tag,
            reason: "length is not a multiple of the element size or misaligned",
        })
    }

    /// UTF-8 文本分節。
    pub fn text(&self, tag: [u8; 4]) -> Result<Text, FormatError> {
        let &(offset, length) = self
            .sections
            .get(&tag)
            .ok_or(FormatError::MissingSection(tag))?;
        Text::mapped(Arc::clone(&self.map), offset, length).ok_or(FormatError::Malformed {
            tag,
            reason: "text is not valid UTF-8",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Writer;

    fn sample(name: &str) -> (tempfile_path::TempPath, Metadata) {
        let metadata = Metadata {
            name: "测试".to_owned(),
            license: "MIT".to_owned(),
            entries: 3,
            ..Metadata::default()
        };
        let path = tempfile_path::TempPath::new(name);
        let numbers: [u32; 3] = [1, 2, 3];
        Writer::new(Kind::Dictionary, &metadata)
            .unwrap()
            .section(*b"NUMS", zerocopy::IntoBytes::as_bytes(&numbers[..]))
            .section(*b"TEXT", "你好 world".as_bytes())
            .write_to(&path.0)
            .unwrap();
        (path, metadata)
    }

    #[test]
    fn round_trips_sections_and_metadata() {
        let (path, metadata) = sample("round-trip");
        assert!(Container::is_qj(&path.0));
        let container = Container::open(&path.0, Kind::Dictionary).unwrap();
        assert_eq!(container.metadata(), &metadata);
        let numbers: Table<u32> = container.table(*b"NUMS").unwrap();
        assert_eq!(&*numbers, &[1, 2, 3]);
        let text = container.text(*b"TEXT").unwrap();
        assert_eq!(&*text, "你好 world");
        assert!(matches!(
            container.bytes(*b"NOPE"),
            Err(FormatError::MissingSection(_))
        ));
        // 3 個 u32 當 u64 表讀：長度不整除
        assert!(matches!(
            container.table::<u64>(*b"NUMS"),
            Err(FormatError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_wrong_kind_and_garbage() {
        let (path, _) = sample("wrong-kind");
        assert!(matches!(
            Container::open(&path.0, Kind::LanguageModel),
            Err(FormatError::WrongKind { .. })
        ));
        let garbage = tempfile_path::TempPath::new("garbage");
        std::fs::write(
            &garbage.0,
            b"not a qj file at all, definitely longer than a header",
        )
        .unwrap();
        assert!(!Container::is_qj(&garbage.0));
        assert!(matches!(
            Container::open(&garbage.0, Kind::Dictionary),
            Err(FormatError::BadMagic)
        ));
    }

    /// 測試用的臨時檔案路徑，drop 時刪除。
    mod tempfile_path {
        use std::path::PathBuf;

        pub struct TempPath(pub PathBuf);

        impl TempPath {
            pub fn new(name: &str) -> Self {
                let dir = std::env::temp_dir().join("qingjian-format-tests");
                std::fs::create_dir_all(&dir).unwrap();
                Self(dir.join(format!("{name}-{}.qj", std::process::id())))
            }
        }

        impl Drop for TempPath {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
    }
}
