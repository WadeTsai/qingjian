//! 整句實驗用的 P2C 打分器：與產品端 `qingjian_neural::P2cScorer` 同一條件，只是推理出錯時直接終止。

use qingjian_core::sentence::SentenceScorer;
use qingjian_neural::CharScorer;

pub struct P2cScorer(pub CharScorer);

impl SentenceScorer for P2cScorer {
    fn score(&self, _context: &str, keys: &str, texts: &[&str]) -> Vec<f64> {
        // 實驗不能靜默回退到基線：任何推理錯誤立即終止，讓不完整結果無法冒充成功。
        self.0
            .score_p2c(keys, texts)
            .expect("P2C evaluation scoring failed")
    }
}
