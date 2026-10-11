#[derive(Debug, thiserror::Error)]
pub enum GlossaryError {
    /// 檔案讀取失敗。
    #[error("failed to read glossary: {0}")]
    Io(#[from] std::io::Error),

    /// 某一行不符合 `詞\t詞性\t譯文` 格式。
    #[error("line {line}: {reason}")]
    Line {
        /// 從 1 開始的行號。
        line: usize,

        /// 具體原因。
        reason: String,
    },

    /// `.qj` 容器層的錯誤。
    #[error("glossary file: {0}")]
    Container(#[from] qingjian_format::FormatError),

    /// `.qj` 分節能讀但內容自相矛盾。
    #[error("corrupt glossary file: {0}")]
    Corrupt(&'static str),
}
