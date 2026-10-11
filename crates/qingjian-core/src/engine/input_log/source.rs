use serde::{Deserialize, Serialize};

use crate::candidate::CandidateKind;

/// 上屏的文字從哪來。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputSource {
    /// 詞庫裡的詞（含使用者詞）。
    Word,

    /// 雲端詞。
    Cloud,

    /// 本地整句轉換。
    Sentence,

    /// 英文候選。
    English,

    /// 快捷候選（日期 / 算式 / 碼點）。
    Shortcut,

    /// 使用者配置的自定義短語。
    Custom,

    /// emoji。
    Emoji,

    /// 回車原樣上屏敲的字母。
    Raw,

    /// 上屏的是候選的譯詞（修飾鍵 + 數字）。
    Translation,

    /// Tab 接受的雲端整句補全。
    CloudSentence,
}

impl From<CandidateKind> for InputSource {
    fn from(kind: CandidateKind) -> Self {
        match kind {
            // 形碼的詞也是詞庫裡的詞，輸入日誌的來源不另分（方案記在別的欄位）
            CandidateKind::Chinese | CandidateKind::Code => Self::Word,
            CandidateKind::Cloud => Self::Cloud,
            // 模型直接生成的也是整句，回放時與詞圖出的整句同一類看
            CandidateKind::Sentence | CandidateKind::Generated => Self::Sentence,
            CandidateKind::English => Self::English,
            CandidateKind::Shortcut => Self::Shortcut,
            CandidateKind::Custom(_) => Self::Custom,
            CandidateKind::Emoji => Self::Emoji,
        }
    }
}
