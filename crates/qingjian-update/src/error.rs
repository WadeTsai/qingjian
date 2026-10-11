use thiserror::Error;

/// 檢查更新失敗的原因；一律只記日誌，不打擾輸入。
#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("runtime failed: {0}")]
    Runtime(#[from] std::io::Error),

    #[error("index is larger than {0} bytes")]
    TooLarge(usize),

    #[error("signature is malformed")]
    MalformedSignature,

    #[error("signature does not match any trusted key")]
    UntrustedSignature,

    #[error("index is not valid JSON: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("index schema {0} is not supported")]
    UnsupportedSchema(u32),
}
