use thiserror::Error;

#[derive(Debug, Error)]
pub enum PredictError {
    #[error("no API key: set `api_key` in config or the `{0}` environment variable")]
    MissingApiKey(String),

    #[error("failed to start async runtime: {0}")]
    Runtime(#[from] std::io::Error),

    #[error("API request failed: {0}")]
    Api(#[from] async_openai::error::OpenAIError),

    #[error("API request timed out after {0} ms")]
    Timeout(u64),

    #[error("API returned no usable content")]
    EmptyReply,

    /// 正文為空且回覆因長度截斷：模型開著思考，輸出額度花在思考上了。
    #[error("API reply was cut off before any content (output budget spent on reasoning)")]
    BudgetExhausted,

    #[error("failed to encode request: {0}")]
    Encode(#[from] serde_json::Error),
}
