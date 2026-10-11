//! 筆畫碼錶：筆畫表（`stroke` 子命令的產物）+ 詞庫 → `codes/stroke.qj`（隨包原生碼錶）。
//!
//! 取碼規則照 `docs/design/aux-code.md` 的「原生表：只帶筆畫」：
//!
//! - 鍵位：CNS 的 1 → `h` 橫、2 → `s` 豎、3 → `p` 撇、5 → `z` 折，點與捺（4 / `n`）→ `n`；
//! - 單字：前 4 筆 + 末筆（不足 5 筆按實際取，1 畫的字就是 1 碼）；
//! - 片語：每字首筆——二字 2 碼、三字 3 碼、四字及以上 = 前三字首筆 + 末字首筆（4 碼）；
//! - 詞裡有一個字不在筆畫表裡，整詞沒有碼、跳過並計入統計：寧可不出，也不用缺字的詞湊半條碼
//!   （四字以上按規則用不到中間的字，同樣要求整詞都在表裡）。
//!
//! 產物是 `AuxCodeTable` 的 `.qj`（輔碼態兩段式查詢用的 `Kind::AuxCodeTable` 容器）；後設資料的預設值
//! （名稱「筆畫」、OFL-1.1、CNS11643 署名）在 `pack` 裡，許可依據見設計文件「許可」一節。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

use qingjian_dictionary::{AuxCodeTable, Dictionary};
use qingjian_format::Metadata;

use crate::error::ConvertError;
use crate::stroke::{malformed, single_char};

#[cfg(test)]
mod tests;

/// 筆畫表：字 → 鍵序列（`h` 橫 / `s` 豎 / `p` 撇 / `n` 點捺 / `z` 折）。
pub struct StrokeTable {
    /// 字 → 鍵序列，序列非空。
    sequences: HashMap<char, String>,
}

impl StrokeTable {
    /// 讀 `stroke` 子命令的產物：每行 `字\t序列`，序列是 `[1235n]+`，`#` 開頭為註釋。
    pub fn from_path(path: &Path) -> Result<Self, ConvertError> {
        Self::parse(path, BufReader::new(File::open(path)?))
    }

    /// 從任意讀入流解析（測試用）。
    fn parse(path: &Path, reader: impl BufRead) -> Result<Self, ConvertError> {
        let mut sequences: HashMap<char, String> = HashMap::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            let number = index + 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(ch), Some(sequence)) = (fields.next(), fields.next()) else {
                return Err(malformed(
                    path,
                    number,
                    "expected a character and a sequence",
                ));
            };
            let Some(ch) = single_char(ch) else {
                return Err(malformed(path, number, "expected one character"));
            };
            let Some(sequence) = key_sequence(sequence) else {
                return Err(malformed(
                    path,
                    number,
                    "expected a stroke sequence of 1 2 3 4 5 n",
                ));
            };
            if sequences.insert(ch, sequence).is_some() {
                return Err(malformed(
                    path,
                    number,
                    "two stroke sequences for one character",
                ));
            }
        }
        Ok(Self { sequences })
    }

    /// 一個字的鍵序列。
    pub fn get(&self, ch: char) -> Option<&str> {
        self.sequences.get(&ch).map(String::as_str)
    }

    /// 表裡的字數。
    pub fn len(&self) -> usize {
        self.sequences.len()
    }

    /// 表是不是空的（空表時任何詞都沒有碼）。
    pub fn is_empty(&self) -> bool {
        self.sequences.is_empty()
    }
}

/// 一次生成的統計。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CodeStats {
    /// 詞庫裡的詞條數（一個詞多條讀音算多條）。
    pub entries: usize,

    /// 去重後的詞數。
    pub words: usize,

    /// 有碼的詞數。
    pub coded: usize,

    /// 詞裡有字不在筆畫表裡、跳過的詞數。
    pub skipped: usize,
}

/// 生成碼錶：讀筆畫表與詞庫，按取碼規則算碼，寫 `.qj`（詞裡缺字的詞跳過）。
pub fn build(
    stroke: &Path,
    dict: &Path,
    out: &Path,
    metadata: &Metadata,
) -> Result<CodeStats, ConvertError> {
    let started = Instant::now();
    let strokes = StrokeTable::from_path(stroke)?;
    tracing::info!(path = %stroke.display(), chars = strokes.len(), "已读笔画表");
    if strokes.is_empty() {
        tracing::warn!(path = %stroke.display(), "笔画表是空的：不会有任何词有码");
    }
    let dictionary = Dictionary::from_path(dict)?;
    let (pairs, stats) = collect(&dictionary, &strokes);
    if stats.words == 0 {
        tracing::warn!(path = %dict.display(), "词库是空的：码表不会有条目");
    }
    let table = AuxCodeTable::from_pairs(pairs)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    table.write_qj(out, metadata)?;

    let size = std::fs::metadata(out).map(|meta| meta.len()).unwrap_or(0);
    tracing::info!(
        out = %out.display(),
        dict = %dict.display(),
        dict_entries = stats.entries,
        words = stats.words,
        coded = stats.coded,
        skipped_no_code = stats.skipped,
        entries = table.len(),
        table_words = table.word_count(),
        size_kb = size / 1_000,
        elapsed_ms = started.elapsed().as_millis(),
        "已生成笔画码表"
    );
    Ok(stats)
}

/// 詞庫 → `(詞, 碼)` 對與統計；同一個詞的重複詞條（多音字）只算一次。
fn collect(dictionary: &Dictionary, strokes: &StrokeTable) -> (Vec<(String, String)>, CodeStats) {
    let mut stats = CodeStats {
        entries: dictionary.len(),
        ..CodeStats::default()
    };
    let mut seen: HashSet<&str> = HashSet::new();
    let mut pairs = Vec::new();
    for entry in dictionary.entries() {
        if !seen.insert(entry.text) {
            continue;
        }
        stats.words += 1;
        match word_code(entry.text, strokes) {
            Some(code) => {
                stats.coded += 1;
                pairs.push((entry.text.to_owned(), code));
            }
            None => stats.skipped += 1,
        }
    }
    (pairs, stats)
}

/// 一個詞的碼：單字走單字規則，片語每字首筆；詞裡有字不在表裡就沒有碼。
fn word_code(word: &str, strokes: &StrokeTable) -> Option<String> {
    let mut sequences = Vec::new();
    for ch in word.chars() {
        sequences.push(strokes.get(ch)?);
    }
    match sequences.as_slice() {
        [] => None,
        [only] => single_char_code(only),
        [a, b] => heads(&[a, b]),
        [a, b, c] => heads(&[a, b, c]),
        many => heads(&[many[0], many[1], many[2], many[many.len() - 1]]),
    }
}

/// 單字碼：前 4 筆 + 末筆；不足 5 筆按實際取。
fn single_char_code(sequence: &str) -> Option<String> {
    let mut keys: Vec<char> = sequence.chars().collect();
    match keys.len() {
        0 => None,
        1..=5 => Some(keys.into_iter().collect()),
        _ => {
            let last = keys.pop()?;
            let mut code: String = keys[..4].iter().collect();
            code.push(last);
            Some(code)
        }
    }
}

/// 片語碼：每個給定字各取首筆。
fn heads(sequences: &[&str]) -> Option<String> {
    sequences
        .iter()
        .map(|sequence| sequence.chars().next())
        .collect()
}

/// 把一個字的 CNS 筆畫序列對映成鍵；認不出的符號返回 `None`。
fn key_sequence(sequence: &str) -> Option<String> {
    if sequence.is_empty() {
        return None;
    }
    sequence.chars().map(key).collect()
}

/// 單個筆畫碼的鍵位（`stroke` 的產物已經把點捺寫成 `n`，`4` 是 CNS 的原始寫法）。
fn key(stroke: char) -> Option<char> {
    match stroke {
        '1' => Some('h'),
        '2' => Some('s'),
        '3' => Some('p'),
        '4' | 'n' => Some('n'),
        '5' => Some('z'),
        _ => None,
    }
}
