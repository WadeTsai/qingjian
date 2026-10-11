use std::ops::Range;
use std::path::Path;

use qingjian_format::{Container, Kind, Metadata, Table, Text, Writer};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::error::DictionaryError;
use crate::matching::Match;
use crate::pattern::{SyllablePattern, canonical_syllable};

/// 收窄到多小的區間就改成逐鍵比對。
const LINEAR_SCAN_LIMIT: usize = 48;

/// `.qj` 裡的分節：詞文本 arena、拼音鍵 arena、鍵索引、詞目。
const TEXT_TAG: [u8; 4] = *b"TEXT";
const KEYS_TAG: [u8; 4] = *b"KEYS";
const INDEX_TAG: [u8; 4] = *b"INDX";
const SLOTS_TAG: [u8; 4] = *b"SLOT";

/// 一條詞目：詞文本在 arena 裡的位置與詞頻。12 位元組、無填充，原樣落盤。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
struct Slot {
    /// 在 `Dictionary::texts` 裡的位元組偏移。
    text_start: u32,

    /// 靜態詞頻。
    frequency: u32,

    /// 位元組長度。
    text_len: u16,

    /// 對齊用，全零。
    reserved: u16,
}

/// 一個拼音鍵（如 `kai fa`）及其下的詞目範圍。16 位元組、無填充，原樣落盤。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
struct KeyIndex {
    /// 在 `Dictionary::keys` 裡的位元組偏移。
    key_start: u32,

    /// 在 `Dictionary::slots` 裡的起始下標。
    first_slot: u32,

    /// 詞目個數（同音詞）。
    slot_count: u32,

    /// 位元組長度。
    key_len: u16,

    /// 對齊用，全零。
    reserved: u16,
}

/// 解析階段的臨時行，排序後才歸併成 `KeyIndex` + `Slot`。
#[derive(Debug, Clone, Copy)]
struct Row {
    key_start: u32,
    key_len: u16,
    slot: Slot,
}

/// 記憶體詞庫。從 TSV 解析時四段資料是自己的，從 `.qj` 開啟時是對映檔案裡的，查詢程式碼不區分。
#[derive(Debug, Default)]
pub struct Dictionary {
    /// 所有詞文本首尾相接。
    texts: Text,

    /// 所有拼音鍵首尾相接（同音詞共用一個鍵）。
    keys: Text,

    /// 按鍵的位元組序升序排列。
    index: Table<KeyIndex>,

    /// 與 `index` 同序；同一個鍵下按詞頻降序。
    slots: Table<Slot>,

    /// 全部詞頻之和，整句轉換把詞頻歸一化成機率時用。
    total_frequency: u64,

    /// `.qj` 裡的來歷（名稱、許可證、署名）；TSV 解析的沒有。
    metadata: Option<Metadata>,
}

impl Dictionary {
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut texts = String::new();
        let mut keys = String::new();
        let mut rows: Vec<Row> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let number = index + 1;
            let mut fields = line.split('\t');
            let text = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing word",
                })?;
            let pinyin = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing pinyin",
                })?;
            let frequency = fields
                .next()
                .map(|s| s.trim().parse::<u32>())
                .transpose()
                .map_err(|_| DictionaryError::Line {
                    line: number,
                    reason: "frequency is not a non-negative integer",
                })?
                .unwrap_or(1);
            let text_len = u16::try_from(text.len()).map_err(|_| DictionaryError::Line {
                line: number,
                reason: "word too long",
            })?;
            let key_len = u16::try_from(pinyin.len()).map_err(|_| DictionaryError::Line {
                line: number,
                reason: "pinyin too long",
            })?;
            rows.push(Row {
                key_start: keys.len() as u32,
                key_len,
                slot: Slot {
                    text_start: texts.len() as u32,
                    frequency,
                    text_len,
                    reserved: 0,
                },
            });
            texts.push_str(text);
            // 規範化：音節之間恰好一個空格
            let mut first = true;
            for syllable in pinyin.split_whitespace() {
                if !first {
                    keys.push(' ');
                }
                keys.push_str(canonical_syllable(syllable));
                first = false;
            }
            let actual = keys.len() as u32 - rows.last().unwrap().key_start;
            rows.last_mut().unwrap().key_len = actual as u16;
        }
        Ok(Self::assemble(texts, keys, rows))
    }

    /// 按鍵排序、同鍵歸併。
    fn assemble(texts: String, keys: String, mut rows: Vec<Row>) -> Self {
        let key_of = |row: &Row| {
            &keys[row.key_start as usize..row.key_start as usize + row.key_len as usize]
        };
        rows.sort_by(|a, b| {
            key_of(a)
                .cmp(key_of(b))
                .then_with(|| b.slot.frequency.cmp(&a.slot.frequency))
        });
        let mut index: Vec<KeyIndex> = Vec::new();
        let mut slots: Vec<Slot> = Vec::with_capacity(rows.len());
        for row in &rows {
            match index.last_mut() {
                Some(last)
                    if key_of(row)
                        == &keys[last.key_start as usize
                            ..last.key_start as usize + last.key_len as usize] =>
                {
                    last.slot_count += 1;
                }
                _ => index.push(KeyIndex {
                    key_start: row.key_start,
                    first_slot: slots.len() as u32,
                    slot_count: 1,
                    key_len: row.key_len,
                    reserved: 0,
                }),
            }
            slots.push(row.slot);
        }
        let total_frequency = slots.iter().map(|s| u64::from(s.frequency)).sum();
        tracing::debug!(entries = slots.len(), keys = index.len(), "词库加载完成");
        Self {
            texts: Text::Owned(texts),
            keys: Text::Owned(keys),
            index: Table::Owned(index),
            slots: Table::Owned(slots),
            total_frequency,
            metadata: None,
        }
    }

    /// 按檔案內容選載入方式：`.qj` 容器直接對映，否則當 TSV 解析。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        if Container::is_qj(path) {
            Self::open_qj(path)
        } else {
            Self::parse(&std::fs::read_to_string(path)?)
        }
    }

    /// 開啟 `.qj` 詞庫：對映四個分節，逐條校驗偏移落在 arena 內且在字元邊界上（壞檔案報錯而不是查詢時 panic）。
    pub fn open_qj(path: &Path) -> Result<Self, DictionaryError> {
        let container = Container::open(path, Kind::Dictionary)?;
        let texts = container.text(TEXT_TAG)?;
        let keys = container.text(KEYS_TAG)?;
        let index: Table<KeyIndex> = container.table(INDEX_TAG)?;
        let slots: Table<Slot> = container.table(SLOTS_TAG)?;
        let mut total_frequency = 0u64;
        for slot in slots.iter() {
            let start = slot.text_start as usize;
            if texts
                .get(start..start + usize::from(slot.text_len))
                .is_none()
            {
                return Err(DictionaryError::Corrupt(
                    "slot points outside the text arena",
                ));
            }
            total_frequency += u64::from(slot.frequency);
        }
        for entry in index.iter() {
            let start = entry.key_start as usize;
            if keys
                .get(start..start + usize::from(entry.key_len))
                .is_none()
            {
                return Err(DictionaryError::Corrupt("key points outside the key arena"));
            }
            let first = entry.first_slot as usize;
            if first + entry.slot_count as usize > slots.len() {
                return Err(DictionaryError::Corrupt(
                    "key points outside the slot table",
                ));
            }
        }
        let has_legacy_keys = index.iter().any(|entry| {
            let start = entry.key_start as usize;
            keys[start..start + usize::from(entry.key_len)]
                .split(' ')
                .any(|syllable| canonical_syllable(syllable) != syllable)
        });
        if has_legacy_keys {
            let metadata = container.metadata().clone();
            let mut owned_keys = String::with_capacity(keys.len());
            let mut rows = Vec::with_capacity(slots.len());
            for entry in index.iter() {
                let start = entry.key_start as usize;
                let key = &keys[start..start + usize::from(entry.key_len)];
                let key_start = owned_keys.len() as u32;
                for (position, syllable) in key.split(' ').enumerate() {
                    if position > 0 {
                        owned_keys.push(' ');
                    }
                    owned_keys.push_str(canonical_syllable(syllable));
                }
                let key_len = (owned_keys.len() as u32 - key_start) as u16;
                let first = entry.first_slot as usize;
                for slot in &slots[first..first + entry.slot_count as usize] {
                    rows.push(Row {
                        key_start,
                        key_len,
                        slot: *slot,
                    });
                }
            }
            let mut dictionary = Self::assemble(texts.to_string(), owned_keys, rows);
            dictionary.metadata = Some(metadata);
            tracing::debug!(
                entries = dictionary.slots.len(),
                keys = dictionary.index.len(),
                "旧词库键已规范化"
            );
            return Ok(dictionary);
        }
        tracing::debug!(
            entries = slots.len(),
            keys = index.len(),
            name = %container.metadata().name,
            "词库已映射"
        );
        Ok(Self {
            texts,
            keys,
            index,
            slots,
            total_frequency,
            metadata: Some(container.metadata().clone()),
        })
    }

    /// 寫成 `.qj`：記憶體裡的四段原樣落盤。`metadata.entries` 會填成詞條數。
    pub fn write_qj(&self, path: &Path, metadata: &Metadata) -> Result<(), DictionaryError> {
        let metadata = Metadata {
            entries: self.slots.len() as u64,
            ..metadata.clone()
        };
        Writer::new(Kind::Dictionary, &metadata)?
            .section(TEXT_TAG, self.texts.as_bytes())
            .section(KEYS_TAG, self.keys.as_bytes())
            .section(INDEX_TAG, self.index.as_bytes())
            .section(SLOTS_TAG, self.slots.as_bytes())
            .write_to(path)?;
        Ok(())
    }

    /// `.qj` 裡的來歷；TSV 解析的返回 `None`。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// 全部詞頻之和。
    pub fn total_frequency(&self) -> u64 {
        self.total_frequency
    }

    /// 全部詞目，按拼音鍵的位元組序、同一個鍵下按詞頻降序。給反查（漢字 → 讀音）建索引用。
    pub fn entries(&self) -> impl Iterator<Item = Match<'_>> + '_ {
        self.index.iter().flat_map(move |entry| {
            let key = self.key(entry);
            let range =
                entry.first_slot as usize..entry.first_slot as usize + entry.slot_count as usize;
            self.slots[range].iter().map(move |slot| Match {
                text: self.text(slot),
                pinyin: key,
                frequency: slot.frequency,
                exact: true,
            })
        })
    }

    /// 只查音節數與模式長度**恰好相等**的詞：整句轉換的詞圖每個格子只要正好覆蓋這幾個音節的詞，
    /// 輸入前綴出候選也只要這種。與 [`Self::lookup_pattern`] 同一套收窄，只是最後一級不收更長的詞、
    /// 簡拼位置只看每個音節塊的第一條鍵，代價與匹配到的音節組合數成正比。
    pub fn lookup_exact(&self, pattern: &[SyllablePattern<'_>]) -> Vec<Match<'_>> {
        let positions: Vec<&[SyllablePattern<'_>]> =
            pattern.iter().map(std::slice::from_ref).collect();
        self.lookup_positions(&positions, true)
    }

    /// 按音節序列查詞。
    ///
    /// - 完整音節：命中音節序列相同的詞，以及以該序列為字首的更長詞。
    /// - `last_is_partial` 為真時，最後一個音節按字首匹配（如 `kai f` 命中 `kai fa`、`kai fang`）。
    pub fn lookup(&self, syllables: &[&str], last_is_partial: bool) -> Vec<Match<'_>> {
        let count = syllables.len();
        let pattern: Vec<SyllablePattern<'_>> = syllables
            .iter()
            .enumerate()
            .map(|(i, s)| SyllablePattern {
                text: s,
                complete: !(last_is_partial && i + 1 == count),
            })
            .collect();
        self.lookup_pattern(&pattern)
    }

    /// 按模式查詞：每個位置可以是完整音節，也可以是字首 / 聲母（簡拼）。
    /// 命中音節數 ≥ 模式長度、逐位滿足模式的詞。
    pub fn lookup_pattern(&self, pattern: &[SyllablePattern<'_>]) -> Vec<Match<'_>> {
        let positions: Vec<&[SyllablePattern<'_>]> =
            pattern.iter().map(std::slice::from_ref).collect();
        self.lookup_positions(&positions, false)
    }

    /// 每個位置給多種寫法（模糊音：`zi` 也接受 `zhi`），命中音節數 ≥ 位置數、逐位滿足其中一種寫法的詞。
    ///
    /// 同一位置的寫法之間不能互相覆蓋（字首 `z` 已包含字首 `zh`，兩者不要同時給；同一寫法不要重複），
    /// 否則命中會重複出現。呼叫方按這個契約生成寫法，這裡不去重。
    pub fn lookup_pattern_alt<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
    ) -> Vec<Match<'_>> {
        self.lookup_positions(positions, false)
    }

    /// [`Self::lookup_exact`] 的多寫法版。
    pub fn lookup_exact_alt<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
    ) -> Vec<Match<'_>> {
        self.lookup_positions(positions, true)
    }

    /// 鍵按位元組序排好，所以「以某段字首開頭的鍵」總是一段連續區間：逐個音節位置用二分收窄區間，
    /// 完整音節直接定位到 `字首 + 音節 + 空格`，簡拼位置按區間裡出現的不同音節跳著掃（每個音節只看一條鍵就能算出它的子區間）。
    /// 代價只和匹配到的音節組合數成正比，與首音節下有多少鍵無關；一個位置的多種寫法逐個走，代價相加而不相乘。
    fn lookup_positions<'p>(
        &self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
        exact_only: bool,
    ) -> Vec<Match<'_>> {
        let mut matches = Vec::new();
        if positions.is_empty() || positions.iter().any(|p| p.as_ref().is_empty()) {
            return matches;
        }
        let mut prefix = String::with_capacity(positions.len() * 6);
        self.narrow(
            positions,
            0,
            &mut prefix,
            0..self.index.len(),
            exact_only,
            &mut matches,
        );
        matches
    }

    /// 在 `range`（其中的鍵都以 `prefix` 開頭，`prefix` 為空或以空格結尾）裡按 `positions[depth..]` 繼續收窄。
    /// `exact_only` 為真時只收音節數正好等於位置數的詞。
    fn narrow<'a, 'p>(
        &'a self,
        positions: &[impl AsRef<[SyllablePattern<'p>]>],
        depth: usize,
        prefix: &mut String,
        range: Range<usize>,
        exact_only: bool,
        out: &mut Vec<Match<'a>>,
    ) {
        let last = depth + 1 == positions.len();
        let base_len = prefix.len();
        for current in positions[depth].as_ref() {
            prefix.truncate(base_len);
            prefix.push_str(canonical_syllable(current.text));
            let sub = self.prefix_range(prefix, range.clone());
            if sub.is_empty() {
                continue;
            }
            // 區間已經很小：直接逐鍵比對剩下的模式，比繼續二分 + 遞迴便宜（全簡拼時小塊極多）。
            // 當前位置只按這一種寫法比（`an` 的區間裡也有 `ang…`，那些留給 `ang` 那一輪，否則重複）
            if sub.len() <= LINEAR_SCAN_LIMIT {
                self.scan_range(
                    current,
                    &positions[depth + 1..],
                    base_len,
                    sub,
                    exact_only,
                    out,
                );
                continue;
            }
            match (current.complete, last) {
                // 完整且是最後一個：正好這個鍵的詞精確命中，`鍵 + 空格` 開頭的更長詞也收
                (true, true) => {
                    if let Some(entry) = self.index[sub.clone()]
                        .first()
                        .filter(|entry| self.key(entry).len() == prefix.len())
                    {
                        self.push_matches(entry, true, out);
                    }
                    if !exact_only {
                        prefix.push(' ');
                        for entry in &self.index[self.prefix_range(prefix, sub)] {
                            self.push_matches(entry, false, out);
                        }
                    }
                }
                (true, false) => {
                    prefix.push(' ');
                    let deeper = self.prefix_range(prefix, sub);
                    self.narrow(positions, depth + 1, prefix, deeper, exact_only, out);
                }
                // 字首且是最後一個、全都要：區間裡全是命中，音節數正好等於位置數的才精確
                (false, true) if !exact_only => {
                    for entry in &self.index[sub] {
                        let exact = !self.key(entry)[base_len..].contains(' ');
                        self.push_matches(entry, exact, out);
                    }
                }
                // 字首且後面還有（或只要精確的）：按區間裡實際出現的音節逐塊進入，
                // 每塊第一條鍵若正好到此為止就是該音節的精確詞
                _ => {
                    let mut position = sub.start;
                    while position < sub.end {
                        let key = self.key(&self.index[position]);
                        let syllable = key[base_len..].split(' ').next().unwrap_or_default();
                        prefix.truncate(base_len);
                        prefix.push_str(syllable);
                        if last {
                            if key.len() == prefix.len() {
                                self.push_matches(&self.index[position], true, out);
                            }
                            prefix.push(' ');
                            position = self
                                .prefix_range(prefix, position..sub.end)
                                .end
                                .max(position + 1);
                            continue;
                        }
                        prefix.push(' ');
                        // 這個音節名下的鍵：先是可能存在的「正好到此為止」的鍵，然後是 `音節 + 空格` 開頭的一段
                        let block = self.prefix_range(prefix, position..sub.end);
                        self.narrow(positions, depth + 1, prefix, block.clone(), exact_only, out);
                        position = block.end.max(position + 1);
                    }
                }
            }
        }
        prefix.truncate(base_len);
    }

    /// 逐鍵比對：`range` 裡每個鍵從位元組偏移 `offset` 起，第一個音節滿足 `first`、後面依次滿足 `rest` 之一寫法的才收。
    fn scan_range<'a, 'p>(
        &'a self,
        first: &SyllablePattern<'p>,
        rest: &[impl AsRef<[SyllablePattern<'p>]>],
        offset: usize,
        range: Range<usize>,
        exact_only: bool,
        out: &mut Vec<Match<'a>>,
    ) {
        for entry in &self.index[range] {
            let mut syllables = self.key(entry)[offset..].split(' ');
            let accepted = syllables.next().is_some_and(|s| first.accepts(s))
                && rest.iter().all(|position| {
                    syllables
                        .next()
                        .is_some_and(|s| position.as_ref().iter().any(|p| p.accepts(s)))
                });
            if accepted {
                let exact = syllables.next().is_none();
                if exact || !exact_only {
                    self.push_matches(entry, exact, out);
                }
            }
        }
    }

    /// `range` 裡以 `prefix` 開頭的鍵所在的子區間（可能為空）。
    fn prefix_range(&self, prefix: &str, range: Range<usize>) -> Range<usize> {
        let entries = &self.index[range.clone()];
        let start = range.start + entries.partition_point(|entry| self.key(entry) < prefix);
        let end = start
            + self.index[start..range.end]
                .partition_point(|entry| self.key(entry).starts_with(prefix));
        start..end
    }

    fn key(&self, entry: &KeyIndex) -> &str {
        &self.keys[entry.key_start as usize..entry.key_start as usize + entry.key_len as usize]
    }

    fn text(&self, slot: &Slot) -> &str {
        &self.texts[slot.text_start as usize..slot.text_start as usize + slot.text_len as usize]
    }

    /// 把一個鍵下的全部詞目追加為命中。
    fn push_matches<'a>(&'a self, entry: &KeyIndex, exact: bool, out: &mut Vec<Match<'a>>) {
        let key = self.key(entry);
        let range =
            entry.first_slot as usize..entry.first_slot as usize + entry.slot_count as usize;
        for slot in &self.slots[range] {
            out.push(Match {
                text: self.text(slot),
                pinyin: key,
                frequency: slot.frequency,
                exact,
            });
        }
    }

    /// 線性掃描版，只做測試基準：與 [`Self::lookup_pattern_alt`] 結果必須一致。
    #[cfg(test)]
    fn lookup_scan<'p>(&self, positions: &[Vec<SyllablePattern<'p>>]) -> Vec<Match<'_>> {
        let mut matches = Vec::new();
        for entry in self.index.iter() {
            let mut syllables = self.key(entry).split(' ');
            let accepted = positions.iter().all(|position| {
                syllables
                    .next()
                    .is_some_and(|s| position.iter().any(|p| p.accepts(s)))
            });
            if accepted {
                let exact = syllables.next().is_none();
                self.push_matches(entry, exact, &mut matches);
            }
        }
        matches
    }
}

#[cfg(test)]
mod tests;
