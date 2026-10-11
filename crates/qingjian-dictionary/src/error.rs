#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    /// 檔案讀取失敗。
    #[error("failed to read dictionary: {0}")]
    Io(#[from] std::io::Error),

    /// 某一行不符合 `詞\t音節\t詞頻` 格式。
    #[error("line {line}: {reason}")]
    Line {
        /// 從 1 開始的行號。
        line: usize,

        /// 具體原因。
        reason: &'static str,
    },

    /// `.qj` 容器層的錯誤（魔數、版本、分節缺失）。
    #[error("dictionary file: {0}")]
    Format(#[from] qingjian_format::FormatError),

    /// `.qj` 分節能讀但內容自相矛盾（偏移越界）。
    #[error("corrupt dictionary file: {0}")]
    Corrupt(&'static str),

    /// 碼錶裡的碼不是 `a-z`、或長度不在 1–8。
    #[error("invalid code: {0:?}")]
    InvalidCode(String),

    /// 一份碼錶裡一條可用的碼都沒有（不是碼錶檔案，或碼列全非法）。
    #[error("no usable code entries in this file")]
    NoCodeEntries,
}
