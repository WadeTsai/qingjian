use std::fmt;

use qingjian_dictionary::{SyllablePattern, canonical_syllable};

/// 切分出的一個音節。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Syllable {
    /// 使用者敲的字母。
    pub text: String,

    /// 是完整音節；否則是聲母（簡拼 `k`）或未打完的字首（`zho`）。
    pub complete: bool,
}

impl Syllable {
    pub fn complete(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            complete: true,
        }
    }

    pub fn partial(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            complete: false,
        }
    }

    pub fn pattern(&self) -> SyllablePattern<'_> {
        SyllablePattern {
            text: canonical_syllable(&self.text),
            complete: self.complete,
        }
    }
}

/// 一種切分方式。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Segmentation {
    /// 音節，順序與輸入一致。
    pub syllables: Vec<Syllable>,
}

impl Segmentation {
    pub fn patterns(&self) -> Vec<SyllablePattern<'_>> {
        self.syllables.iter().map(Syllable::pattern).collect()
    }

    /// 覆蓋的輸入字母數（不含 `'`）。
    pub fn letters(&self) -> usize {
        self.syllables.iter().map(|s| s.text.len()).sum()
    }

    pub fn incomplete_count(&self) -> usize {
        self.syllables.iter().filter(|s| !s.complete).count()
    }

    pub fn last_is_partial(&self) -> bool {
        self.syllables.last().is_some_and(|s| !s.complete)
    }

    /// 用分隔符連線各音節，給 marked text 用：`kai'fa`、`k'f`。
    pub fn joined(&self, separator: &str) -> String {
        self.syllables
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(separator)
    }
}

impl fmt::Display for Segmentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, syllable) in self.syllables.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            f.write_str(&syllable.text)?;
            if !syllable.complete {
                f.write_str("…")?;
            }
        }
        Ok(())
    }
}
