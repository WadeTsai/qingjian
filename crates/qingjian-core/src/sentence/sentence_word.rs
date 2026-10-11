/// 整句路徑上的一個詞。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentenceWord {
    /// 詞。
    pub text: String,

    /// 詞的全拼音節。
    pub syllables: Vec<String>,

    /// 是佔位音節（詞庫裡連單字都沒有，用拼音本身頂著），不是詞庫裡的詞。
    pub placeholder: bool,
}
