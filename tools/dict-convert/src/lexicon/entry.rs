/// 一條待寫出的詞條。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconEntry {
    /// 詞。
    pub text: String,

    /// 音節，不帶聲調，ü 寫 v。
    pub syllables: Vec<String>,

    /// 詞頻。
    pub frequency: u32,
}
