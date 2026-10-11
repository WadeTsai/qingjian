use std::collections::HashMap;
use std::sync::Arc;

use qingjian_dictionary::{SyllablePattern, canonical_syllable};

use super::word::SpanWord;

/// 快取最多存多少個格子，超過就整個清掉（一次整句最多幾百個格子，正常輸入遠到不了）。
pub const MAX_SPAN_CACHE_ENTRIES: usize = 8192;

/// 詞圖格子候選的快取：鍵是格子的模式（各位置的寫法與是否完整），值是排好、截好的候選。
///
/// 敲鍵是增量的：第 n+1 鍵只新增以它結尾的幾個格子，其餘格子上一鍵都算過，而格子查詞是整句轉換裡最貴的一步
/// （每個簡拼位置都要在詞庫裡逐音節塊收窄）。候選與詞庫、使用者詞、使用者選擇次數、個人出現次數有關，
/// 這些一變（上屏 / 學習）就整個清掉，由 Engine 負責。
#[derive(Debug, Default)]
pub struct SpanCache {
    /// 格子模式 → 候選。
    entries: HashMap<String, Arc<[SpanWord]>>,
}

impl SpanCache {
    /// 格子的快取鍵：每個位置的各種寫法用 `|` 連、字首寫法帶 `…`，位置之間空格。
    pub fn key(span: &[Vec<SyllablePattern<'_>>]) -> String {
        let mut key = String::with_capacity(span.len() * 8);
        for (index, position) in span.iter().enumerate() {
            if index > 0 {
                key.push(' ');
            }
            for (alt, pattern) in position.iter().enumerate() {
                if alt > 0 {
                    key.push('|');
                }
                key.push_str(canonical_syllable(pattern.text));
                if !pattern.complete {
                    key.push('…');
                }
            }
        }
        key
    }

    /// 有就直接給，沒有就算一次存起來。
    pub fn get_or_insert_with(
        &mut self,
        key: String,
        compute: impl FnOnce() -> Vec<SpanWord>,
    ) -> Arc<[SpanWord]> {
        if let Some(found) = self.entries.get(&key) {
            return Arc::clone(found);
        }
        if self.entries.len() >= MAX_SPAN_CACHE_ENTRIES {
            self.entries.clear();
        }
        let words: Arc<[SpanWord]> = compute().into();
        self.entries.insert(key, Arc::clone(&words));
        words
    }

    /// 詞庫、使用者詞或學習資料變了：全部作廢。
    pub fn clear(&mut self) {
        self.entries.clear();
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
    fn key_distinguishes_completeness_and_alternatives() {
        let full = vec![vec![SyllablePattern::complete("kai")]];
        let prefix = vec![vec![SyllablePattern::prefix("kai")]];
        let fuzzy = vec![vec![
            SyllablePattern::prefix("z"),
            SyllablePattern::prefix("zh"),
        ]];
        assert_eq!(SpanCache::key(&full), "kai");
        assert_eq!(SpanCache::key(&prefix), "kai…");
        assert_eq!(SpanCache::key(&fuzzy), "z…|zh…");
        assert_ne!(SpanCache::key(&full), SpanCache::key(&prefix));
    }

    #[test]
    fn second_lookup_is_served_from_the_cache() {
        let mut cache = SpanCache::default();
        let mut computed = 0;
        let mut fetch = |cache: &mut SpanCache| {
            cache.get_or_insert_with("kai fa".to_owned(), || {
                computed += 1;
                vec![SpanWord {
                    text: "开发".to_owned(),
                    syllables: vec!["kai".to_owned(), "fa".to_owned()],
                    frequency: 9,
                    penalty: 0.0,
                }]
            })
        };
        let first = fetch(&mut cache);
        let second = fetch(&mut cache);
        assert_eq!(first, second);
        assert_eq!(computed, 1);
        assert_eq!(cache.len(), 1);
        cache.clear();
        assert!(cache.is_empty());
    }
}
