//! 候選視窗的一行：序號、候選詞、annotation 片段。

use super::tone::Tone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 顯示用序號文本，如 `1`。
    pub index: String,

    /// 候選詞。
    pub text: String,

    /// 緊跟在候選詞後面的輔碼，如 `[kf]`：碼是詞本身的屬性，不進右側的 annotation 列。
    pub code: Option<String>,

    /// 右側 annotation，按順序繪製；沒有譯文時為空。
    pub annotation: Vec<(String, Tone)>,

    /// 來自雲聯想：詞前畫一個小云朵，與本地候選區分。
    pub cloud: bool,
}

impl Row {
    /// 只有序號和候選詞的一行。
    pub fn plain(index: usize, text: impl Into<String>) -> Self {
        Self {
            index: (index + 1).to_string(),
            text: text.into(),
            code: None,
            annotation: Vec::new(),
            cloud: false,
        }
    }
}
