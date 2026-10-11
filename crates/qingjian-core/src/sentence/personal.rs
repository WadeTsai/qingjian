use super::{Context, Interpolation, UserNgram};

/// 整句轉換與詞級排序用到的個人部分：個人 n-gram（沒有就是 `None`）與它同靜態模型插值的引數。
/// 打包成一個值往 Viterbi 裡傳，免得每層簽名都多一個引數。
#[derive(Debug, Clone, Copy)]
pub struct Personal<'a> {
    /// 個人 n-gram；沒學習資料時為 `None`，打分原樣用靜態模型。
    pub ngram: Option<&'a UserNgram>,

    /// 插值引數。
    pub interpolation: Interpolation,
}

impl<'a> Personal<'a> {
    /// 沒有個人資料。
    pub const NONE: Personal<'static> = Personal {
        ngram: None,
        interpolation: Interpolation::DEFAULT,
    };

    /// 用預設插值引數包一個個人 n-gram。
    pub fn new(ngram: Option<&'a UserNgram>) -> Self {
        Self {
            ngram,
            interpolation: Interpolation::DEFAULT,
        }
    }

    /// `word` 在個人資料裡出現過幾次；沒有個人資料就是 0。
    pub fn count(&self, word: &str) -> u32 {
        self.ngram.map_or(0, |ngram| ngram.count(word))
    }

    /// 把靜態模型給出的 `log P(word | previous)` 與個人機率插值；沒有個人資料原樣返回。
    pub fn blend(&self, context: Context<'_>, word: &str, base_log_prob: f64) -> f64 {
        self.ngram.map_or(base_log_prob, |ngram| {
            ngram.blend(context, word, base_log_prob, &self.interpolation)
        })
    }
}
