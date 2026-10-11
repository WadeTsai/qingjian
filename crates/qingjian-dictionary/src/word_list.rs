use std::collections::HashMap;
use std::path::Path;

use crate::error::DictionaryError;

/// 英文詞表：按小寫編碼查原樣大小寫的詞，也能按字首補全。
/// 檔案格式 TSV：`詞\t編碼[\t詞頻]`（編碼預設為詞的小寫，詞頻預設 0）。
#[derive(Debug, Default)]
pub struct WordList {
    /// 小寫編碼 → 在 `entries` 裡的下標。
    by_code: HashMap<String, usize>,

    /// (編碼, 原樣詞, 詞頻)，按編碼位元組序排好，字首補全二分定位。
    entries: Vec<(String, String, u32)>,
}

impl WordList {
    pub fn parse(source: &str) -> Result<Self, DictionaryError> {
        let mut by_code: HashMap<String, usize> = HashMap::new();
        let mut entries: Vec<(String, String, u32)> = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let word = fields
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(DictionaryError::Line {
                    line: index + 1,
                    reason: "missing word",
                })?;
            let code = fields
                .next()
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_lowercase)
                .unwrap_or_else(|| word.to_ascii_lowercase());
            let frequency = fields
                .next()
                .map(|s| s.trim().parse::<u32>())
                .transpose()
                .map_err(|_| DictionaryError::Line {
                    line: index + 1,
                    reason: "frequency is not a non-negative integer",
                })?
                .unwrap_or(0);
            // 同一編碼取第一個（詞表按常用度排序時即最常用的寫法）
            if by_code.contains_key(&code) {
                continue;
            }
            by_code.insert(code.clone(), entries.len());
            entries.push((code, word.to_owned(), frequency));
        }
        // 按編碼排序後重建下標
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (index, (code, _, _)) in entries.iter().enumerate() {
            by_code.insert(code.clone(), index);
        }
        tracing::debug!(words = entries.len(), "英文词表加载完成");
        Ok(Self { by_code, entries })
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// 輸入（已小寫）對應的英文詞。
    pub fn get(&self, code: &str) -> Option<&str> {
        self.by_code
            .get(code)
            .map(|&index| self.entries[index].1.as_str())
    }

    /// 輸入（已小寫）對應的詞頻；詞表裡沒有為 `None`。
    pub fn frequency(&self, code: &str) -> Option<u32> {
        self.by_code.get(code).map(|&index| self.entries[index].2)
    }

    /// 以 `prefix` 開頭（不含正好相等的）的詞裡詞頻最高的 `limit` 個，按詞頻降序。
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<&str> {
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        let start = self
            .entries
            .partition_point(|(code, _, _)| code.as_str() < prefix);
        let mut hits: Vec<&(String, String, u32)> = self.entries[start..]
            .iter()
            .take_while(|(code, _, _)| code.starts_with(prefix))
            .filter(|(code, _, _)| code != prefix)
            .collect();
        hits.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        hits.into_iter()
            .take(limit)
            .map(|(_, word, _)| word.as_str())
            .collect()
    }

    /// 全部詞目：(小寫編碼, 原樣詞, 詞頻)，按編碼位元組序。
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, u32)> {
        self.entries
            .iter()
            .map(|(code, word, frequency)| (code.as_str(), word.as_str(), *frequency))
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
    fn looks_up_by_lowercase_code() {
        let list = WordList::parse("GitHub\tgithub\nhello\thello\niPhone\n").unwrap();
        assert_eq!(list.get("github"), Some("GitHub"));
        assert_eq!(list.get("iphone"), Some("iPhone"));
        assert_eq!(list.get("hello"), Some("hello"));
        assert_eq!(list.get("nope"), None);
    }

    #[test]
    fn completes_prefixes_by_frequency() {
        let list = WordList::parse(
            "compass\tcompass\t300\ncompany\tcompany\t900\ncompare\tcompare\t500\ncom\tcom\t100\ncomma\tcomma\n",
        )
        .unwrap();
        assert_eq!(list.complete("comp", 2), ["company", "compare"]);
        assert_eq!(
            list.complete("com", 10),
            ["company", "compare", "compass", "comma"]
        );
        assert!(list.complete("zzz", 3).is_empty());
        assert!(list.complete("", 3).is_empty());
    }
}
