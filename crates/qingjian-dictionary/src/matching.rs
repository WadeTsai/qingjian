/// 一次查詢命中的詞目，全部借用自詞庫。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<'a> {
    /// 詞。
    pub text: &'a str,

    /// 空格分隔的音節。
    pub pinyin: &'a str,

    /// 靜態詞頻。
    pub frequency: u32,

    /// 音節數與查詢模式的長度一致（而不是以查詢為字首的更長詞）。
    pub exact: bool,
}

impl<'a> Match<'a> {
    pub fn syllables(&self) -> impl Iterator<Item = &'a str> {
        self.pinyin.split(' ')
    }

    pub fn syllable_count(&self) -> usize {
        self.pinyin.split(' ').count()
    }
}
