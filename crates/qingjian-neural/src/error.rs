use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NeuralError {
    #[error("io error reading {path}: {source}")]
    Io {
        /// 出錯的檔案。
        path: PathBuf,

        /// 底層錯誤。
        source: std::io::Error,
    },

    #[error("bad json in {path}: {source}")]
    Json {
        /// 出錯的檔案。
        path: PathBuf,

        /// 底層錯誤。
        source: serde_json::Error,
    },

    #[error("no model in {0}: neither a .qjm file nor model.safetensors")]
    NotFound(PathBuf),

    #[error("bad .qjm container: {0}")]
    Format(#[from] qingjian_format::FormatError),

    #[error("model error: {0}")]
    Candle(#[from] candle_core::Error),

    #[error("corrupt model: {0}")]
    Corrupt(&'static str),
}
