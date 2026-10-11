use super::cloud_word::CloudWord;

/// 聯想結果。只是候選之外的補充展示，**不重排本地候選**：雲端詞補進第一頁末尾幾格。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prediction {
    /// 對應的請求序號。
    pub sequence: u64,

    /// 這段拼音可能對應的詞（已按拼音校驗）。
    pub words: Vec<CloudWord>,

    /// 組句中的整句補全，替換整段拼音。
    pub sentence: Option<String>,
}

impl Prediction {
    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.sentence.is_none()
    }
}
