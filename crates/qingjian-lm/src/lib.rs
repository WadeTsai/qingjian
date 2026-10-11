//! 整句轉換用的詞級語言模型：實現 Core 的 `LanguageModel` trait。
//! 從 TSV 載入一元 / 二元計數（`from_paths`），或直接對映 `.qj`（`from_path`，`dict-convert pack lm` 生成，啟動近零耗時）。
//!
//! TSV 由 `tools/dict-convert bigram` 從語料統計得到：
//!
//! ```text
//! lm-unigram.tsv    詞\t計數        （`<s>` 是句首標記，計數為句子數）
//! lm-bigram.tsv     前詞\t後詞\t計數
//! ```
//!
//! 機率是插值平滑：P(w|v) = λ·c(v,w)/c(v) + (1-λ)·c(w)/N。模型裡沒有的詞交回 Core 用詞庫詞頻兜底。

mod bigram_model;
mod error;
mod successor;
mod word_entry;

pub use bigram_model::{BigramModel, SENTENCE_START};
pub use error::LmError;
