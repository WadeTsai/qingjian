use qingjian_core::sentence::SentenceScorer;

use crate::{CharScorer, P2c};

/// 字級模型：條件是游標前文，忽略按鍵。
impl SentenceScorer for CharScorer {
    fn score(&self, context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        match CharScorer::score(self, context, texts) {
            Ok(scores) => scores,
            Err(error) => {
                tracing::warn!(%error, "神经重打分失败，本次不用");
                Vec::new()
            }
        }
    }
}

/// P2C 模型：條件是使用者敲的那段按鍵，忽略前文。
///
/// 凍結集 8322 句上比字級模型的重排高 0.69 個點（41.80% → 42.49%，配對 McNemar p=0.003），
/// 延遲也低（中位 18.5 對 22.0 ms），而且同一個模型還能造詞——所以整句重排換成它。
/// 留出驗證過 λ 沒有過擬合：0.5 / 0.75 / 1.0 在兩半上都贏，只有 0.25 會輸。
pub struct P2cScorer(pub CharScorer);

impl P2cScorer {
    /// 字表裡沒有 `<sep>` 的不是 P2C 模型，建不出來。
    pub fn new(scorer: CharScorer) -> Option<Self> {
        scorer.vocab().sep()?;
        Some(Self(scorer))
    }
}

impl SentenceScorer for P2cScorer {
    fn score(&self, _context: &str, keys: &str, texts: &[&str]) -> Vec<f64> {
        match self.0.score_p2c(keys, texts) {
            Ok(scores) => scores,
            Err(error) => {
                tracing::warn!(%error, "P2C 重打分失败，本次不用");
                Vec::new()
            }
        }
    }

    fn generate(&self, keys: &str, beam: usize, max_chars: usize) -> Vec<String> {
        let Some(decoder) = P2c::new(self.0.model(), self.0.vocab()) else {
            return Vec::new();
        };
        match decoder.convert(keys, beam, max_chars) {
            Ok(items) => items.into_iter().map(|item| item.text).collect(),
            Err(error) => {
                tracing::warn!(%error, "P2C 生成失败，本次不用");
                Vec::new()
            }
        }
    }
}
