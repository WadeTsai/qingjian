use serde::{Deserialize, Serialize};

use super::JapaneseSense;

/// 一箇中文詞的釋義：詞性、英文譯詞、日文譯詞、越南文譯詞。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossEntry {
    /// 中文詞。
    pub word: String,

    /// 詞性縮寫（`n.` / `v.` …，Core `PartOfSpeech` 認得的那套）；模型給不出就沒有。
    pub pos: Option<String>,

    /// 英文譯詞，最多兩個，按常用度。
    pub en: Vec<String>,

    /// 日文譯詞，最多兩個，按常用度。
    pub ja: Vec<JapaneseSense>,

    /// 越南文譯詞，最多兩個，按常用度。
    #[serde(default)]
    pub vi: Vec<String>,
}

impl GlossEntry {
    /// 所有學習語言都沒給出譯詞的條目沒有用。
    pub fn is_useful(&self) -> bool {
        !self.en.is_empty() || !self.ja.is_empty() || !self.vi.is_empty()
    }
}
