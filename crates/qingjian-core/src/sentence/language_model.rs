/// 整句轉換的打分來源：詞級語言模型。實現放兄弟 crate（`qingjian-lm`），Core 只認這個 trait。
pub trait LanguageModel: Send {
    /// `log P(word | previous)`；`previous` 為 `None` 表示句首。模型不認識 `word` 時返回 `None`，
    /// 由 Core 用詞庫詞頻兜底。
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64>;
}

/// 沒接語言模型：一律兜底，整句轉換退化為一元詞頻。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoLanguageModel;

impl LanguageModel for NoLanguageModel {
    fn log_prob(&self, _previous: Option<&str>, _word: &str) -> Option<f64> {
        None
    }
}
