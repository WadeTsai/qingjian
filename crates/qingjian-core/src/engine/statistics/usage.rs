use std::ops::{Add, AddAssign};

use crate::sentence::is_han;

/// 一次上屏折成的用量；加起來就是一段時間的用量。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    /// 漢字數：上屏文字裡的漢字，標點、字母、emoji 不算。
    pub hanzi: u64,

    /// 中文詞數：選一個詞算一個，整句按切出來的詞數算，譯詞 / 英文 / 快捷候選不算。
    pub words: u64,

    /// 英文詞數：英文候選、原樣上屏的英文詞、英文譯詞按空格分開數。
    pub english_words: u64,

    /// 上屏次數。
    pub commits: u64,
}

impl Usage {
    /// 按上屏的文字本身數漢字與英文詞，算一次上屏；中文詞數由呼叫方按來源定（見 `Engine`）。
    pub fn of_text(text: &str) -> Self {
        Self {
            hanzi: text.chars().filter(|c| is_han(*c)).count() as u64,
            words: 0,
            english_words: count_english_words(text),
            commits: 1,
        }
    }

    /// 一次都沒記過。
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

impl AddAssign for Usage {
    fn add_assign(&mut self, other: Self) {
        self.hanzi += other.hanzi;
        self.words += other.words;
        self.english_words += other.english_words;
        self.commits += other.commits;
    }
}

impl Add for Usage {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        self += other;
        self
    }
}

/// 拉丁字母組成的詞有幾個：`don't`、`open-source` 各算一個，純數字不算。
fn count_english_words(text: &str) -> u64 {
    text.split(|c: char| !(c.is_ascii_alphabetic() || c == '\'' || c == '-'))
        .filter(|token| token.chars().any(|c| c.is_ascii_alphabetic()))
        .count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_hanzi_and_english_words_separately() {
        let usage = Usage::of_text("你好，world！");
        assert_eq!(
            usage,
            Usage {
                hanzi: 2,
                words: 0,
                english_words: 1,
                commits: 1
            }
        );
        assert_eq!(Usage::of_text("open-source don't 2024").english_words, 2);
        assert_eq!(Usage::of_text("😀").hanzi, 0);
    }

    #[test]
    fn adds_up() {
        let mut total = Usage::default();
        assert!(total.is_empty());
        total += Usage::of_text("你好");
        total += Usage::of_text("world");
        assert_eq!(total.hanzi, 2);
        assert_eq!(total.english_words, 1);
        assert_eq!(total.commits, 2);
        assert!(!total.is_empty());
    }
}
