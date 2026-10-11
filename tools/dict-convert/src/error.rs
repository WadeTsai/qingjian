use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Dictionary(#[from] qingjian_dictionary::DictionaryError),

    #[error(transparent)]
    LanguageModel(#[from] qingjian_lm::LmError),

    #[error(transparent)]
    Glossary(#[from] qingjian_translate::GlossaryError),

    #[error(transparent)]
    Neural(#[from] qingjian_neural::NeuralError),

    /// `pack` 少了必填的後設資料（只有 `codes` 有預設值）。
    #[error("pack {kind} needs --name")]
    MissingName {
        /// `pack` 的種類名。
        kind: &'static str,
    },

    /// 抽樣對照出現白名單之外的不符。
    #[error("stroke verification failed: {unmatched} character(s) differ outside the whitelist")]
    Verify {
        /// 不符的字數。
        unmatched: usize,
    },

    /// 檔案不是預期格式。
    #[error("{path}:{line}: {reason}")]
    Format {
        /// 出錯的檔案。
        path: PathBuf,

        /// 從 1 開始的行號。
        line: usize,

        /// 具體原因。
        reason: String,
    },
}
