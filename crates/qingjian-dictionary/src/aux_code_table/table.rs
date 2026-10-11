//! 碼錶本體：詞 → 碼的對映、`.qj` 讀寫與查詢。

use std::ops::Range;
use std::path::Path;

use qingjian_format::{Container, Kind, Metadata, Table, Text, Writer, hash};

use super::entry::CodeEntry;
use super::lookup::AuxCodeLookup;
use super::parsed::ParsedTable;
use crate::error::DictionaryError;

/// `.qj` 裡的分節：詞文本 arena、碼鍵 arena、條目表、詞 → 條目區間的雜湊索引。
const TEXT_TAG: [u8; 4] = *b"TEXT";
const CODE_TAG: [u8; 4] = *b"CODE";
const ENTR_TAG: [u8; 4] = *b"ENTR";
const HASH_TAG: [u8; 4] = *b"HASH";

/// 碼的最長位元組數（`a-z`，1–8）。
pub const MAX_CODE_LEN: usize = 8;

/// 碼是否合法：全是 `a-z`（大小寫不敏感）、長度 1–`MAX_CODE_LEN`。
pub fn is_valid_code(code: &str) -> bool {
    !code.is_empty() && code.len() <= MAX_CODE_LEN && code.bytes().all(|b| b.is_ascii_lowercase())
}

/// 碼錶：詞 → 碼的對映。從 `.qj` 開啟時四段資料都是對映檔案裡的一段，查詢程式碼不區分。
#[derive(Debug, Default)]
pub struct AuxCodeTable {
    /// 所有詞文本首尾相接。
    words: Text,

    /// 所有碼首尾相接。
    codes: Text,

    /// 按詞文本的位元組序升序；同一個詞的條目相鄰。
    entries: Table<CodeEntry>,

    /// 詞 → 條目區間起點的雜湊索引（`hash::build` 的開放定址表）。
    index: Table<u32>,

    /// 去重後的詞數。
    word_count: usize,

    /// `.qj` 裡的來歷（名稱、許可證、署名）；記憶體裡造的沒有。
    metadata: Option<Metadata>,
}

impl AuxCodeTable {
    /// 用 `(詞, 碼)` 對造一張表。
    ///
    /// 重複的 `(詞, 碼)` 去掉；同一個詞的多條碼都保留（一詞多碼）。碼非法時報錯，
    /// 呼叫方（匯入）應當先過濾並統計，不要讓壞資料走到這裡。
    pub fn from_pairs(
        pairs: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, DictionaryError> {
        let mut rows: Vec<(String, String)> = pairs.into_iter().collect();
        for (_, code) in &rows {
            if !is_valid_code(code) {
                return Err(DictionaryError::InvalidCode(code.clone()));
            }
        }
        rows.sort();
        rows.dedup();
        let mut words = String::new();
        let mut codes = String::new();
        let mut entries: Vec<CodeEntry> = Vec::with_capacity(rows.len());
        let mut previous: Option<&str> = None;
        let mut word_count = 0;
        for (word, code) in &rows {
            if previous != Some(word.as_str()) {
                word_count += 1;
                previous = Some(word.as_str());
            }
            let word_len = u16::try_from(word.len())
                .map_err(|_| DictionaryError::Corrupt("word too long for a code table"))?;
            entries.push(CodeEntry {
                word_start: words.len() as u32,
                code_start: codes.len() as u32,
                word_len,
                code_len: code.len() as u8,
                reserved: 0,
            });
            words.push_str(word);
            codes.push_str(code);
        }
        let index = hash::build(entries.len(), |id| {
            let entry = entries[id as usize];
            &words[entry.word_start as usize..entry.word_start as usize + entry.word_len as usize]
        });
        tracing::debug!(entries = entries.len(), words = word_count, "码表已建立");
        Ok(Self {
            words: Text::Owned(words),
            codes: Text::Owned(codes),
            entries: Table::Owned(entries),
            index: Table::Owned(index),
            word_count,
            metadata: None,
        })
    }

    /// 從 TSV 解析：每行 `詞\t碼`，`#` 開頭為註釋，空行忽略；同一個詞可以有多行。
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut pairs = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let number = index + 1;
            let mut fields = line.split('\t');
            let word = fields
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing word",
                })?;
            let code = fields
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing code",
                })?;
            if !is_valid_code(code) {
                return Err(DictionaryError::InvalidCode(code.to_owned()));
            }
            pairs.push((word.to_owned(), code.to_owned()));
        }
        Self::from_pairs(pairs)
    }

    /// 按檔案內容選載入方式：`.qj` 容器直接對映；文本再按內容分 Rime `.dict.yaml` 與 `詞\t碼` TSV。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::open(path)
        } else {
            Self::from_text(&std::fs::read_to_string(path)?)
        }
    }

    /// 解析碼錶文本：有 YAML 頭（`---` 或 `name:` 打頭）的按 Rime 規則讀——與「匯入碼錶」走同一套
    /// 解析（`columns` / `import_tables` / 「有詞無碼」統計口徑一致），其餘按 `詞\t碼` 的 TSV 讀。
    pub fn from_text(text: &str) -> Result<Self, DictionaryError> {
        if !looks_like_rime(text) {
            return Self::parse(text);
        }
        let parsed = ParsedTable::parse(text);
        if parsed.pairs.is_empty() {
            return Err(DictionaryError::NoCodeEntries);
        }
        Self::from_pairs(parsed.pairs)
    }

    /// 開啟 `.qj` 碼錶：對映四個分節，逐條校驗偏移落在 arena 內、條目按詞有序、雜湊索引自洽。
    pub fn open(path: &Path) -> Result<Self, DictionaryError> {
        let container = Container::open(path, Kind::AuxCodeTable)?;
        let words = container.text(TEXT_TAG)?;
        let codes = container.text(CODE_TAG)?;
        let entries: Table<CodeEntry> = container.table(ENTR_TAG)?;
        let index: Table<u32> = container.table(HASH_TAG)?;
        if !hash::is_valid(&index, entries.len()) {
            return Err(DictionaryError::Corrupt(
                "code table hash index is malformed",
            ));
        }
        let mut word_count = 0;
        let mut previous: Option<&str> = None;
        for entry in entries.iter() {
            let start = entry.word_start as usize;
            let word = words
                .get(start..start + usize::from(entry.word_len))
                .ok_or(DictionaryError::Corrupt(
                    "code entry points outside the word arena",
                ))?;
            let start = entry.code_start as usize;
            let code = codes
                .get(start..start + usize::from(entry.code_len))
                .ok_or(DictionaryError::Corrupt(
                    "code entry points outside the code arena",
                ))?;
            if !is_valid_code(code) {
                return Err(DictionaryError::Corrupt("code is not a-z of length 1-8"));
            }
            if previous.is_none_or(|last| last < word) {
                word_count += 1;
            } else if previous != Some(word) {
                return Err(DictionaryError::Corrupt(
                    "code entries are not sorted by word",
                ));
            }
            previous = Some(word);
        }
        tracing::debug!(
            entries = entries.len(),
            words = word_count,
            name = %container.metadata().name,
            "码表已映射"
        );
        Ok(Self {
            words,
            codes,
            entries,
            index,
            word_count,
            metadata: Some(container.metadata().clone()),
        })
    }

    /// 寫成 `.qj`：記憶體裡的四段原樣落盤。`metadata.entries` 會填成詞條數。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), DictionaryError> {
        let metadata = Metadata {
            entries: self.entries.len() as u64,
            ..metadata.clone()
        };
        Writer::new(Kind::AuxCodeTable, &metadata)?
            .section(TEXT_TAG, self.words.as_bytes())
            .section(CODE_TAG, self.codes.as_bytes())
            .section(ENTR_TAG, self.entries.as_bytes())
            .section(HASH_TAG, self.index.as_bytes())
            .write_to(path)?;
        Ok(())
    }

    /// `.qj` 裡的來歷；記憶體裡造的返回 `None`。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    /// 詞條數（詞 → 碼 的條目數，一詞多碼算多條）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 去重後的詞數。
    pub fn word_count(&self) -> usize {
        self.word_count
    }

    /// `word` 的全部碼，按檔案裡的順序。沒有這個詞時是空迭代器。
    pub fn codes_of<'a>(&'a self, word: &str) -> impl Iterator<Item = &'a str> + 'a {
        let range = self.range_of(word);
        self.entries[range].iter().map(|entry| self.code(entry))
    }

    /// 全部條目，按詞文本的位元組序升序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.entries
            .iter()
            .map(|entry| (self.word(entry), self.code(entry)))
    }

    /// 詞 → 條目區間。沒有這個詞時是空區間。
    fn range_of(&self, word: &str) -> Range<usize> {
        let Some(first) = hash::find(&self.index, word, |id| {
            self.word(&self.entries[id as usize])
        }) else {
            return 0..0;
        };
        let start = first as usize;
        let length = self.entries[start..].partition_point(|entry| self.word(entry) == word);
        start..start + length
    }

    fn word(&self, entry: &CodeEntry) -> &str {
        let start = entry.word_start as usize;
        &self.words[start..start + usize::from(entry.word_len)]
    }

    fn code(&self, entry: &CodeEntry) -> &str {
        let start = entry.code_start as usize;
        &self.codes[start..start + usize::from(entry.code_len)]
    }
}

/// 文本像不像 Rime 詞庫 / 碼錶：第一條非註釋非空行是 `---` 或以 `name:` 打頭。
fn looks_like_rime(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .is_some_and(|line| line == "---" || line.starts_with("name:"))
}

impl AuxCodeLookup for AuxCodeTable {
    fn code_with_prefix<'a>(&'a self, word: &str, prefix: &str) -> Option<&'a str> {
        // inherent 的 codes_of 是迭代器版本，這裡要用它（別遞歸回 trait）
        AuxCodeTable::codes_of(self, word).find(|code| code.starts_with(prefix))
    }

    fn codes_of<'a>(&'a self, word: &str) -> Vec<&'a str> {
        AuxCodeTable::codes_of(self, word).collect()
    }
}
