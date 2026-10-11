//! 漢字 → 全拼：按語言模型把句子切成詞（[`segment_text`]），每個詞查詞庫裡的讀音，
//! 詞庫沒有的詞退到逐字查；多音字取詞庫裡那個詞（或那個字最常用）的讀音。任何一個字查不到就放棄這句。

use std::collections::HashMap;

use qingjian_core::sentence::{LanguageModel, segment_text};
use qingjian_dictionary::Dictionary;

/// 漢字到讀音的反查表。
pub struct Transcriber {
    /// 詞文本 → 無分隔全拼；同一個詞多個讀音時取詞頻最高的。
    readings: HashMap<String, (String, u32)>,
}

impl Transcriber {
    /// 從詞庫（主詞庫 + 附加詞庫）建反查表。
    pub fn new<'a>(dictionaries: impl IntoIterator<Item = &'a Dictionary>) -> Self {
        let mut readings: HashMap<String, (String, u32)> = HashMap::new();
        for dictionary in dictionaries {
            for entry in dictionary.entries() {
                let pinyin: String = entry.pinyin.split(' ').collect();
                match readings.get_mut(entry.text) {
                    Some(best) if best.1 >= entry.frequency => {}
                    Some(best) => *best = (pinyin, entry.frequency),
                    None => {
                        readings.insert(entry.text.to_owned(), (pinyin, entry.frequency));
                    }
                }
            }
        }
        Self { readings }
    }

    pub fn len(&self) -> usize {
        self.readings.len()
    }

    /// 一句純漢字的全拼；有字查不到讀音就 `None`。
    pub fn transcribe(&self, text: &str, model: &dyn LanguageModel) -> Option<String> {
        let words: Vec<String> = match segment_text(text, model) {
            Some(clauses) => clauses.into_iter().flatten().collect(),
            None => text.chars().map(String::from).collect(),
        };
        let mut pinyin = String::new();
        for word in &words {
            match self.readings.get(word) {
                Some((reading, _)) => pinyin.push_str(reading),
                None => {
                    for c in word.chars() {
                        let (reading, _) = self.readings.get(c.encode_utf8(&mut [0; 4]))?;
                        pinyin.push_str(reading);
                    }
                }
            }
        }
        Some(pinyin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::sentence::NoLanguageModel;

    const SAMPLE: &str =
        "长\tzhang\t900\n长\tchang\t800\n大\tda\t1000\n长度\tchang du\t500\n度\tdu\t700\n";

    #[test]
    fn prefers_the_word_reading_over_the_most_common_char_reading() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let transcriber = Transcriber::new([&dictionary]);
        // 沒有語言模型時逐字：長 取詞頻高的 zhang
        assert_eq!(
            transcriber.transcribe("长大", &NoLanguageModel).as_deref(),
            Some("zhangda")
        );
        // 詞庫有「長度」這個詞：整詞查到 chang
        struct Model;
        impl LanguageModel for Model {
            fn log_prob(&self, _: Option<&str>, word: &str) -> Option<f64> {
                (word == "长度").then_some(-3.0)
            }
        }
        assert_eq!(
            transcriber.transcribe("长度", &Model).as_deref(),
            Some("changdu")
        );
        assert_eq!(transcriber.transcribe("长短", &NoLanguageModel), None);
    }
}
