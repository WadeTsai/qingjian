//! 形碼碼錶：按編碼查字詞（五筆）。
//!
//! 與 [`crate::Dictionary`] 的區別在鍵：詞庫按拼音音節序列查，碼錶按編碼本身查，
//! 敲的編碼是詞的編碼的字首即可命中（`gan` 命中編碼 `gant` 的 開發），沒有音節也沒有切分。
//!
//! 檔案格式 TSV：
//!
//! ```text
//! 詞\t編碼\t詞頻
//! 開發\tgant\t9000
//! ```
//!
//! `#` 開頭為註釋行，空行忽略。編碼統一按小寫存，查詢輸入也須已小寫。

use std::path::Path;

use crate::error::DictionaryError;
use crate::matching::Match;

/// 一條碼錶詞目。
#[derive(Debug, Clone)]
struct Entry {
    /// 全碼，小寫。
    code: String,

    /// 詞。
    text: String,

    /// 靜態詞頻。
    frequency: u32,
}

/// 形碼碼錶。詞目按 `(編碼, 詞頻降序)` 排好，同字首的是一段連續區間。
///
/// `Clone` 是給回放用的：`Engine::set_code_table` 收所有權，而回放要在方案之間來回切，
/// 手裡得留一份（整份日誌通常只有一套方案，切的次數很少）。
#[derive(Debug, Default, Clone)]
pub struct CodeTable {
    /// 按編碼位元組序升序，同一個編碼下按詞頻降序。
    entries: Vec<Entry>,

    /// 全部詞頻之和，上下文得分的兜底用。
    total_frequency: u64,
}

impl CodeTable {
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut entries: Vec<Entry> = Vec::new();
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
            let code = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: number,
                    reason: "missing code",
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
            entries.push(Entry {
                code: code.to_ascii_lowercase(),
                text: text.to_owned(),
                frequency,
            });
        }
        entries.sort_by(|a, b| {
            a.code
                .cmp(&b.code)
                .then_with(|| b.frequency.cmp(&a.frequency))
                .then_with(|| a.text.cmp(&b.text))
        });
        // 同一個詞記了兩遍（不同碼錶版本合併）留詞頻高的那條，排完序就是靠前的那條
        entries.dedup_by(|a, b| a.code == b.code && a.text == b.text);
        let total_frequency = entries.iter().map(|e| u64::from(e.frequency)).sum();
        tracing::debug!(entries = entries.len(), "码表加载完成");
        Ok(Self {
            entries,
            total_frequency,
        })
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// 編碼正好等於輸入，或輸入是它的字首的詞，最多 `limit` 條。
    ///
    /// 編碼與輸入相等（這個詞打全了）的排在最前，其餘按詞頻降序；`limit` 之後的不返回。
    /// 單字母輸入（`g`，一級簡碼）的字首區間有上萬條，所以先按 `limit` 線性選出前面一段再排序，
    /// 不是把整個區間排完再截斷——`lookup` 在每次按鍵的路徑上。
    pub fn lookup(&self, code: &str, limit: usize) -> Vec<Match<'_>> {
        if code.is_empty() || limit == 0 {
            return Vec::new();
        }
        let order = |a: &&Entry, b: &&Entry| {
            (b.code == code)
                .cmp(&(a.code == code))
                .then_with(|| b.frequency.cmp(&a.frequency))
                .then_with(|| a.code.cmp(&b.code))
                .then_with(|| a.text.cmp(&b.text))
        };
        let start = self.entries.partition_point(|e| e.code.as_str() < code);
        let mut hits: Vec<&Entry> = self.entries[start..]
            .iter()
            .take_while(|e| e.code.starts_with(code))
            .collect();
        if hits.len() > limit {
            hits.select_nth_unstable_by(limit, order);
            hits.truncate(limit);
        }
        hits.sort_by(order);
        hits.into_iter()
            .map(|entry| Match {
                text: entry.text.as_str(),
                pinyin: entry.code.as_str(),
                frequency: entry.frequency,
                exact: entry.code == code,
            })
            .collect()
    }

    /// 全部詞頻之和。
    pub fn total_frequency(&self) -> u64 {
        self.total_frequency
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_by_prefix_and_prefers_the_exact_code() {
        let table =
            CodeTable::parse("开发\tgant\t900\n开\tga\t5000\n一\tggll\t100000\n个\twhj\t8000\n")
                .unwrap();
        // 打全了的編碼排在字首命中的詞前面，哪怕字首詞詞頻更高
        let hits = table.lookup("ga", 10);
        assert_eq!(
            hits.iter().map(|h| (h.text, h.exact)).collect::<Vec<_>>(),
            [("开", true), ("开发", false)]
        );
        // 沒打全的編碼只按詞頻
        let hits = table.lookup("g", 10);
        assert_eq!(
            hits.iter().map(|h| h.text).collect::<Vec<_>>(),
            ["一", "开", "开发"]
        );
        assert!(hits.iter().all(|h| !h.exact));
        assert!(table.lookup("zzz", 10).is_empty());
        assert!(table.lookup("", 10).is_empty());
        assert!(table.lookup("ga", 0).is_empty());
    }

    #[test]
    fn truncates_to_the_limit_keeping_the_best() {
        // 命中條數多於 limit 時先線性選一段再排，結果要與整體排序的前幾條一致
        let table =
            CodeTable::parse("甲\tg\t10\n乙\tg\t50\n丙\tg\t30\n丁\tg\t40\n戊\tg\t20\n").unwrap();
        assert_eq!(
            table
                .lookup("g", 3)
                .iter()
                .map(|h| h.text)
                .collect::<Vec<_>>(),
            ["乙", "丁", "丙"]
        );
        assert_eq!(table.lookup("g", 9).len(), 5);
        assert_eq!(table.lookup("g", 5).len(), 5);
    }

    #[test]
    fn keeps_the_highest_frequency_of_a_duplicated_entry() {
        let table = CodeTable::parse("开发\tgant\t900\n开发\tgant\t3000\n").unwrap();
        assert_eq!(table.len(), 1);
        assert_eq!(table.lookup("gant", 10)[0].frequency, 3000);
    }

    #[test]
    fn rejects_lines_without_a_code() {
        assert!(CodeTable::parse("开发\n").is_err());
        assert!(CodeTable::parse("开发\tgant\t不是数字\n").is_err());
    }

    #[test]
    fn normalizes_codes_to_lowercase() {
        let table = CodeTable::parse("开\tGA\t5000\n").unwrap();
        assert_eq!(table.lookup("ga", 10)[0].text, "开");
    }
}
