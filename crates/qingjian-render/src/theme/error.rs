//! 主題檔案讀不進來。

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    /// 讀不到 `theme.json`。
    #[error("cannot read theme file: {0}")]
    Io(#[from] std::io::Error),

    /// 不是合法的 JSON，或結構對不上（缺必填項、型別錯）。
    #[error("invalid theme file: {0}")]
    Json(#[from] serde_json::Error),

    /// `extends` 指向的主題不存在。
    #[error("unknown base theme {id:?}")]
    UnknownBase { id: String },

    /// `extends` 鏈太長（多半是互相繼承）。
    #[error("theme inheritance too deep at {id:?}")]
    TooDeep { id: String },
}
