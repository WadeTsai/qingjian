//! Unihan 裡每個字的普通話讀音：kHanyuPinlu（帶頻次）、kXHC1983（《現代漢語詞典》1983 全部讀音）、kMandarin（首選讀音）。

use std::collections::HashMap;
use std::path::Path;

use super::tone::strip_tone;
use crate::error::ConvertError;

/// 一個字的讀音資料。
#[derive(Debug, Default, Clone)]
struct Entry {
    /// kMandarin 的首選讀音。
    mandarin: Option<String>,

    /// kHanyuPinlu：讀音 → 頻次。
    pinlu: Vec<(String, u32)>,

    /// kXHC1983 的全部讀音，按詞典出現順序。
    xhc: Vec<String>,
}

/// 字 → 讀音。
#[derive(Debug, Default)]
pub struct CharReadings {
    /// 每個字的資料。
    entries: HashMap<char, Entry>,
}

impl CharReadings {
    /// 讀 `Unihan_Readings.txt`，只取三個欄位。
    pub fn load(path: &Path) -> Result<Self, ConvertError> {
        let source = std::fs::read_to_string(path)?;
        let mut entries: HashMap<char, Entry> = HashMap::new();
        for line in source.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(code), Some(field), Some(value)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if !matches!(field, "kMandarin" | "kHanyuPinlu" | "kXHC1983") {
                continue;
            }
            let Some(ch) = code
                .strip_prefix("U+")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .and_then(char::from_u32)
            else {
                continue;
            };
            let entry = entries.entry(ch).or_default();
            match field {
                // 兩個值時第一個是大陸讀音
                "kMandarin" => entry.mandarin = value.split_whitespace().next().map(strip_tone),
                // `xíng(2943) háng(218)`
                "kHanyuPinlu" => {
                    entry.pinlu = value
                        .split_whitespace()
                        .filter_map(|item| {
                            let (reading, count) = item.split_once('(')?;
                            let count: u32 = count.trim_end_matches(')').parse().ok()?;
                            Some((strip_tone(reading), count))
                        })
                        .collect();
                }
                // `0442.080:háng 0443.050,0443.060:hàng`
                "kXHC1983" => {
                    entry.xhc = value
                        .split_whitespace()
                        .filter_map(|item| item.split_once(':').map(|(_, r)| strip_tone(r)))
                        .collect();
                }
                _ => {}
            }
        }
        Ok(Self { entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 這個字的全部讀音，按可信度排：kHanyuPinlu 頻次高的在前，然後 kMandarin，然後 kXHC1983 其餘的。去重。
    pub fn all(&self, ch: char) -> Vec<String> {
        let Some(entry) = self.entries.get(&ch) else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        let mut pinlu = entry.pinlu.clone();
        pinlu.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        for (reading, _) in pinlu {
            if !out.contains(&reading) {
                out.push(reading);
            }
        }
        if let Some(mandarin) = &entry.mandarin
            && !out.contains(mandarin)
        {
            out.push(mandarin.clone());
        }
        for reading in &entry.xhc {
            if !out.contains(reading) {
                out.push(reading.clone());
            }
        }
        out
    }

    /// 單字詞條用的讀音與權重：有 kHanyuPinlu 就按頻次，沒有的讀音按主讀音的 `minor_share`；
    /// 沒有 kHanyuPinlu 時首選讀音 1.0、其餘 `minor_share`。
    pub fn weighted(&self, ch: char, minor_share: f64) -> Vec<(String, f64)> {
        let all = self.all(ch);
        let Some(entry) = self.entries.get(&ch) else {
            return Vec::new();
        };
        let top = entry.pinlu.iter().map(|(_, n)| *n).max().unwrap_or(0);
        all.into_iter()
            .enumerate()
            .map(|(index, reading)| {
                let weight = entry
                    .pinlu
                    .iter()
                    .find(|(r, _)| *r == reading)
                    .map(|(_, n)| f64::from(*n))
                    .unwrap_or(if top > 0 {
                        f64::from(top) * minor_share
                    } else if index == 0 {
                        1.0
                    } else {
                        minor_share
                    });
                (reading, weight)
            })
            .collect()
    }

    /// 一個詞的標註是否每個字都落在該字的已知讀音裡（LLM 標註的校驗）。
    pub fn accepts_word(&self, text: &str, syllables: &[String]) -> bool {
        let chars: Vec<char> = text.chars().collect();
        chars.len() == syllables.len()
            && chars
                .iter()
                .zip(syllables)
                .all(|(ch, syllable)| self.all(*ch).iter().any(|r| r == syllable))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_three_fields_and_orders_by_evidence() {
        let dir = std::env::temp_dir().join("qingjian-unihan-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("readings.txt");
        std::fs::write(
            &path,
            "# comment\nU+884C\tkHanyuPinlu\txíng(2943) háng(218)\nU+884C\tkMandarin\txíng\n\
U+884C\tkXHC1983\t0442.080:háng 0443.050:hàng 0460.010:héng 1290.030:xíng\nU+5973\tkMandarin\tnǚ\n",
        )
        .unwrap();
        let readings = CharReadings::load(&path).unwrap();
        assert_eq!(
            readings.all('行'),
            ["xing", "hang", "hang", "heng"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .into_iter()
                .fold(Vec::new(), |mut v, s| {
                    if !v.contains(&s) {
                        v.push(s)
                    }
                    v
                })
        );
        assert_eq!(readings.all('女'), ["nv"]);
        let weighted = readings.weighted('行', 0.05);
        assert_eq!(weighted[0].0, "xing");
        assert!(weighted[0].1 > weighted[1].1);
        // 長 沒載入進來，整詞校驗不過
        assert!(!readings.accepts_word("行长", &["hang".to_owned(), "zhang".to_owned()]));
        assert!(readings.accepts_word("女", &["nv".to_owned()]));
    }
}
