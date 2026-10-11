//! 語料統計的一元詞頻（`bigram` 子命令寫的 lm-unigram.tsv：`詞\t次數`）。

use std::collections::HashMap;
use std::path::Path;

use qingjian_dictionary::canonical_syllable;

use crate::error::ConvertError;

pub fn load(path: &Path) -> Result<HashMap<String, u64>, ConvertError> {
    Ok(rows(path)?.into_iter().collect())
}

/// 同上，保持檔案順序、允許重複。
pub fn rows(path: &Path) -> Result<Vec<(String, u64)>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    Ok(source
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (word, count) = l.split_once('\t')?;
            Some((word.trim().to_owned(), count.trim().parse().ok()?))
        })
        .collect())
}

/// `--extra-words` 檔案的一行：`詞\t次數[\t拼音]`，拼音是空格分隔的音節（短語層由成分詞拼出，給了就不再按字猜）。
pub struct ExtraWord {
    /// 詞。
    pub text: String,

    /// 語料次數，當詞頻。
    pub count: u64,

    /// 讀音；沒給為 `None`。
    pub syllables: Option<Vec<String>>,
}

/// 讀額外詞檔案，保持順序。
pub fn extra_words(path: &Path) -> Result<Vec<ExtraWord>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    Ok(source
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut fields = l.split('\t');
            let text = fields.next()?.trim().to_owned();
            let count = fields.next()?.trim().parse().ok()?;
            let syllables = fields
                .next()
                .map(|p| {
                    p.split_whitespace()
                        .map(|s| canonical_syllable(s).to_owned())
                        .collect::<Vec<_>>()
                })
                .filter(|s| !s.is_empty());
            Some(ExtraWord {
                text,
                count,
                syllables,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_given_pinyin() {
        let path = std::env::temp_dir().join(format!(
            "qingjian-extra-words-normalize-{}.tsv",
            std::process::id()
        ));
        std::fs::write(&path, "策略\t20\tce lue\n虐待\t10\tnue dai\n").unwrap();
        let words = extra_words(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(words[0].syllables.as_deref().unwrap(), ["ce", "lve"]);
        assert_eq!(words[1].syllables.as_deref().unwrap(), ["nve", "dai"]);
    }
}
