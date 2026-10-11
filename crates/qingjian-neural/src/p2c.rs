//! 帶噪拼音 → 漢字（beam search）。
//!
//! 訓練序列是 `[帶噪拼音] <sep> [漢字] <eos>`，所以解碼就是：把拼音和 `<sep>` 喂進去當前綴，
//! 然後逐字生成到 `<eos>`。字首的 K/V 算一次存下來，每敲一鍵只需把新字母接上去。
//!
//! 字首開頭要帶一個 `<eos>`：訓練流裡每個樣本的拼音前面都跟著上一個樣本的 `<eos>`，
//! 少了它模型會當成在續寫上文、把短拼音看成被截斷的尾巴，於是把正確的詞放句尾再補一堆字首
//! （`jiekou` → 我們可以使用多個介面）。

use candle_core::Tensor;

use crate::vocab::{EOS, Vocab};
use crate::{CharLm, NeuralError};

/// 一條候選。
#[derive(Debug, Clone)]
pub struct Candidate {
    pub text: String,

    /// 生成這段漢字的 log 機率之和（越接近 0 越好）。
    pub score: f64,
}

/// P2C 解碼器：借用已載入的模型與字表。
pub struct P2c<'a> {
    model: &'a CharLm,
    vocab: &'a Vocab,
    sep: u32,
}

impl<'a> P2c<'a> {
    /// 字表裡沒有 `<sep>`（是字級 LM 的模型）就返回 `None`。
    pub fn new(model: &'a CharLm, vocab: &'a Vocab) -> Option<Self> {
        let sep = vocab.sep()?;
        Some(Self { model, vocab, sep })
    }

    /// 拼音鍵 → 前 `beam` 條候選，按分數降序。`max_chars` 是生成長度上限。
    pub fn convert(
        &self,
        keys: &str,
        beam: usize,
        max_chars: usize,
    ) -> Result<Vec<Candidate>, NeuralError> {
        let beam = beam.max(1);
        let mut prefix = Vec::with_capacity(keys.len() + 2);
        prefix.push(EOS);
        prefix.extend(self.vocab.encode(keys));
        prefix.push(self.sep);

        let context = self.model.config().context;
        if prefix.len() + max_chars > context {
            return Err(NeuralError::Corrupt(
                "含章·通变的拼音和生成长度超出模型上下文",
            ));
        }

        let device = self.model.device();
        let split = prefix.len() - 1;
        // 字首最後一位留給 step 走，這樣不用為空快取分支
        let mut cache = self.model.prefix_cache(&prefix[..split])?;
        let last = Tensor::from_vec(prefix[split..].to_vec(), (1, 1), device)?;
        let (mut log_probs, mut grown) = self.model.step(&cache, &last)?;

        let mut beams: Vec<(f64, Vec<u32>)> = vec![(0.0, Vec::new())];
        let mut done: Vec<(f64, Vec<u32>)> = Vec::new();

        for _ in 0..max_chars {
            let rows = log_probs.to_vec2::<f32>()?;
            let mut pool: Vec<(f64, usize, u32)> = Vec::new();
            for (index, (row, (score, _))) in rows.iter().zip(&beams).enumerate() {
                for (token, &lp) in top_k(row, beam) {
                    pool.push((score + f64::from(lp), index, token));
                }
            }
            pool.sort_by(|a, b| b.0.total_cmp(&a.0));

            let mut next: Vec<(f64, Vec<u32>)> = Vec::new();
            let mut order: Vec<u32> = Vec::new();
            let mut tokens: Vec<u32> = Vec::new();
            for (score, index, token) in pool {
                if token == EOS {
                    done.push((score, beams[index].1.clone()));
                    continue;
                }
                if next.len() >= beam {
                    continue;
                }
                let mut text = beams[index].1.clone();
                text.push(token);
                next.push((score, text));
                order.push(index as u32);
                tokens.push(token);
            }
            if next.is_empty() || done.len() >= beam {
                beams = next;
                break;
            }

            let width = next.len();
            let picked = Tensor::from_vec(order, width, device)?;
            cache = grown.select(&picked)?;
            let idx = Tensor::from_vec(tokens, (width, 1), device)?;
            let stepped = self.model.step(&cache, &idx)?;
            log_probs = stepped.0;
            grown = stepped.1;
            beams = next;
        }

        // 沒走到 <eos> 的也收進來，短拼音常常 beam 還沒耗完就夠了
        done.extend(beams);
        done.sort_by(|a, b| b.0.total_cmp(&a.0));

        let mut seen = Vec::new();
        let mut out = Vec::new();
        for (score, ids) in done {
            let text = self.vocab.decode(&ids);
            if text.is_empty() || seen.contains(&text) {
                continue;
            }
            seen.push(text.clone());
            out.push(Candidate { text, score });
            if out.len() >= beam {
                break;
            }
        }
        Ok(out)
    }
}

/// 一行 log 機率裡最大的 `k` 個，返回 `(token, log 機率)`。
fn top_k(row: &[f32], k: usize) -> Vec<(u32, &f32)> {
    let mut best: Vec<(u32, &f32)> = Vec::with_capacity(k + 1);
    for (token, lp) in row.iter().enumerate() {
        if best.len() < k {
            best.push((token as u32, lp));
            if best.len() == k {
                best.sort_by(|a, b| b.1.total_cmp(a.1));
            }
        } else if lp > best[k - 1].1 {
            best[k - 1] = (token as u32, lp);
            best.sort_by(|a, b| b.1.total_cmp(a.1));
        }
    }
    if best.len() < k {
        best.sort_by(|a, b| b.1.total_cmp(a.1));
    }
    best
}
