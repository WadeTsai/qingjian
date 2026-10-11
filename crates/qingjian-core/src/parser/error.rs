#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    /// 緩衝區為空，沒有可切分的內容。
    #[error("empty input")]
    Empty,

    /// 出現了拼音字母和 `'` 之外的字元。
    #[error("character {ch:?} at position {position} is not a pinyin letter")]
    InvalidChar {
        /// 從 1 開始的字元位置。
        position: usize,

        /// 違規字元。
        ch: char,
    },

    /// 沒有任何一種切法能覆蓋整個輸入。
    #[error("cannot segment input into valid syllables")]
    NoSegmentation,
}
