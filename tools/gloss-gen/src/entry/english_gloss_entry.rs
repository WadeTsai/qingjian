use serde::{Deserialize, Serialize};

/// 一個英文詞的中文釋義（英文候選右側顯示），JSONL 裡一行一個。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnglishGlossEntry {
    /// 英文詞，按詞表裡的寫法。
    pub word: String,

    /// 詞性縮寫（Core `PartOfSpeech` 認得的那套）；模型給不出就沒有。
    pub pos: Option<String>,

    /// 中文釋義，最多兩個，按常用度。
    pub zh: Vec<String>,
}
