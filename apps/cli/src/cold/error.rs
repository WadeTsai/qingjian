//! 冷啟動實驗錯誤：失敗必須中斷，不能靜默跳過難例。

#[derive(Debug, thiserror::Error)]
pub enum ColdError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Neural(#[from] qingjian_neural::NeuralError),

    #[error("invalid cold evaluation row: {0}")]
    Invalid(String),

    #[error("Hanzhang Tongbian model requires a <sep> token in its vocabulary")]
    Model,
}
