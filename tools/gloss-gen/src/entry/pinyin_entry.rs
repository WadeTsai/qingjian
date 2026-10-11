use serde::{Deserialize, Serialize};

/// 一箇中文詞的拼音標註（多音字按詞義定讀音），JSONL 裡一行一個。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinyinEntry {
    /// 中文詞。
    pub word: String,

    /// 每個字一個音節，不帶聲調，ü 寫 v（`chong qing`）。
    pub pinyin: Vec<String>,
}
