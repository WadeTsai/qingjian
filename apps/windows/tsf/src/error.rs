use qingjian_platform::protocol::CodecError;

/// 與 Server 通訊時的錯誤。
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// 幀編解碼 / 底層 IO 出錯。
    #[error("codec: {0}")]
    Codec(#[from] CodecError),

    /// 等應答時對端在幀邊界關閉了連線。
    #[error("server closed the connection")]
    Closed,

    /// 收到了與當前請求不匹配的訊息。
    #[error("unexpected server message: {0}")]
    Unexpected(&'static str),
}
