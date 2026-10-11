//! 挖詞結果的過濾：連續單欄位裡數出來的子串大多不是詞（「的一」「了一個」），按兩條規則篩。
//!
//! 1. 虛詞規則：以助詞 / 副詞類虛字開頭的不要（「的話」「瞭解」這種正常詞早在詞庫裡，落到這裡的都是碎片）；
//!    全由虛字組成的不要；同一個字重複的不要（「哈哈哈」這類疊詞另算）。
//! 2. 相鄰字對的點互資訊（PMI）：詞裡每對相鄰字 `ab` 的 `ln(p(ab) / (p(a) p(b)))` 取最小值，低於閾值的說明只是碰巧挨著。
//!    `p(a)` 用語言模型一元表裡單字的次數，`p(ab)` 用挖詞本身數出來的兩欄位次數（同一份語料）；
//!    某對相鄰字沒被數到（次數不夠 `min_count`）直接不要。閾值 3 是 2026-09-05 在 8.3 萬候選上人工抽樣定的：
//!    2–3 之間開始混進「回家吧」「看到了」一類。

use std::collections::HashMap;
use std::path::Path;

use crate::error::ConvertError;

/// 開頭不能是這些字（助詞、語氣詞、常見副詞、介詞）。
const LEADING_FUNCTION_CHARS: &str = "的了着得地吗吧呢啊呀哦嗯也都就还才又再很太被把";

/// 全由這些字組成的不算詞。
const FUNCTION_CHARS: &str = "的了着得地吗吧呢啊呀哦嗯也都就还才又再很太被把是在有和这那我你他她它们不没要会能可以与及或而且但";

/// 過濾器：單字次數 + 閾值。
pub struct OovFilter {
    /// 單字 → 語料裡的次數（從一元表讀，只留單字）。
    unigram: HashMap<char, u64>,

    /// 一元表的總次數（不含句首標記）。
    total: f64,

    /// 相鄰字對 PMI 的下限。
    min_pmi: f64,
}

impl OovFilter {
    /// 讀語言模型一元表（`詞\t次數`，`<s>` 是句首標記）。
    pub fn from_unigram_file(path: &Path, min_pmi: f64) -> Result<Self, ConvertError> {
        Ok(Self::from_unigram_text(
            &std::fs::read_to_string(path)?,
            min_pmi,
        ))
    }

    pub fn from_unigram_text(text: &str, min_pmi: f64) -> Self {
        let mut unigram = HashMap::new();
        let mut total = 0.0;
        for line in text.lines() {
            if line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(word), Some(count)) = (fields.next(), fields.next()) else {
                continue;
            };
            let Ok(count) = count.parse::<u64>() else {
                continue;
            };
            if word == "<s>" {
                continue;
            }
            total += count as f64;
            let mut chars = word.chars();
            if let (Some(c), None) = (chars.next(), chars.next()) {
                unigram.insert(c, count);
            }
        }
        Self {
            unigram,
            total,
            min_pmi,
        }
    }

    /// 虛詞規則：過不了的直接不要，不用算 PMI。
    pub fn passes_function_rules(word: &str) -> bool {
        let mut chars = word.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if LEADING_FUNCTION_CHARS.contains(first) {
            return false;
        }
        if word.chars().all(|c| FUNCTION_CHARS.contains(c)) {
            return false;
        }
        if word.chars().all(|c| c == first) {
            return false;
        }
        true
    }

    /// 詞裡相鄰字對 PMI 的最小值；某對沒在 `pair_counts` 裡為 `None`。
    pub fn min_pair_pmi(&self, word: &str, pair_counts: &HashMap<String, u32>) -> Option<f64> {
        let chars: Vec<char> = word.chars().collect();
        let mut lowest: Option<f64> = None;
        for pair in chars.windows(2) {
            let key: String = pair.iter().collect();
            let joint = f64::from(*pair_counts.get(&key)?);
            let a = *self.unigram.get(&pair[0]).unwrap_or(&1) as f64;
            let b = *self.unigram.get(&pair[1]).unwrap_or(&1) as f64;
            let pmi = ((joint / self.total) / ((a / self.total) * (b / self.total))).ln();
            lowest = Some(lowest.map_or(pmi, |l: f64| l.min(pmi)));
        }
        lowest
    }

    /// 是否留下：虛詞規則 + PMI 閾值。
    pub fn keeps(&self, word: &str, pair_counts: &HashMap<String, u32>) -> bool {
        Self::passes_function_rules(word)
            && self
                .min_pair_pmi(word, pair_counts)
                .is_some_and(|pmi| pmi >= self.min_pmi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_rules_reject_fragments() {
        assert!(!OovFilter::passes_function_rules("的话"));
        assert!(!OovFilter::passes_function_rules("了一个"));
        assert!(!OovFilter::passes_function_rules("我们的"));
        assert!(!OovFilter::passes_function_rules("哈哈哈"));
        assert!(OovFilter::passes_function_rules("回家"));
        assert!(OovFilter::passes_function_rules("有点"));
    }

    #[test]
    fn pmi_keeps_words_whose_characters_cling_together() {
        // 100 萬字的語料：回 1 萬、家 2 萬、吧 5 萬；回家 8 千，家吧 60
        let filter = OovFilter::from_unigram_text(
            "<s>\t999\n回\t10000\n家\t20000\n吧\t50000\n其他\t920000\n",
            3.0,
        );
        let mut pairs = HashMap::new();
        pairs.insert("回家".to_owned(), 8000);
        pairs.insert("家吧".to_owned(), 60);
        let pmi = filter.min_pair_pmi("回家", &pairs).unwrap();
        assert!(pmi > 3.0, "{pmi}");
        assert!(filter.keeps("回家", &pairs));
        // 回家吧 取最小的一對（家吧），碰巧挨著，PMI 低
        assert!(filter.min_pair_pmi("回家吧", &pairs).unwrap() < 1.0);
        assert!(!filter.keeps("回家吧", &pairs));
        // 沒數到的字對直接不要
        assert_eq!(filter.min_pair_pmi("回去", &pairs), None);
        assert!(!filter.keeps("回去", &pairs));
    }
}
