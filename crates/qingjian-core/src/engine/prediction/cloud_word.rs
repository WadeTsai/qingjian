use crate::candidate::{Candidate, CandidateKind};

/// 雲端給出的一個詞候選：文本加全拼音節，音節用來校驗它確實對得上使用者敲的拼音，也用來記成使用者詞。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloudWord {
    /// 詞。
    pub text: String,

    /// 全拼音節，如 `["zhang", "tao"]`。問字模式的答案不校驗拼音，這裡為空。
    pub syllables: Vec<String>,

    /// 顯示用讀音（問字模式答案的帶聲調拼音，如 `sēn`）。
    pub reading: Option<String>,
}

impl CloudWord {
    /// 轉成雲端來源的候選（譯文留給 `Engine::annotate` 補）。
    pub fn into_candidate(self) -> Candidate {
        Candidate {
            text: self.text,
            kind: CandidateKind::Cloud,
            syllables: self.syllables,
            reading: self.reading,
            translation: None,
            aux_code: None,
        }
    }
}
