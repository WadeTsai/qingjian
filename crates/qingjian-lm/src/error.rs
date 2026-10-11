use thiserror::Error;

#[derive(Debug, Error)]
pub enum LmError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bad line {line} in {file}: {reason}")]
    Format {
        /// 檔名。
        file: &'static str,

        /// 行號（從 1 數）。
        line: usize,

        /// 出錯原因。
        reason: &'static str,
    },

    #[error("language model file: {0}")]
    Container(#[from] qingjian_format::FormatError),

    #[error("corrupt language model file: {0}")]
    Corrupt(&'static str),
}
