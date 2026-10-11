use crate::candidate::Translation;

/// 雲端寫好的一條釋義。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledGloss {
    /// 中文詞。
    pub word: String,

    /// 學習語言的譯詞（最多兩條）。
    pub translation: Translation,
}
