//! 字級 Transformer 語言模型的本地推理（candle）。
//!
//! 載入匯出的模型——開發時是三件套目錄（`model.safetensors` + `config.json` + `vocab.json`），
//! 隨包與使用者目錄裡是打成一個檔案的 `.qjm`（見 [`qjm`]）——給「游標前文 + 候選文本」按字累加 log 機率，供 Core 給整句路徑重打分。
//! 結構是 GPT-2 風格 decoder-only（pre-LN、erf GELU、可學習位置嵌入、輸入輸出嵌入共享），張量名與訓練指令碼約定一致。
//!
//! 預設 CPU；`metal` feature 走 Apple GPU。

mod config;
mod core_scorer;
mod error;
mod model;
mod p2c;
pub mod qjm;
mod scorer;
mod vocab;

pub use config::ModelConfig;
pub use core_scorer::P2cScorer;
pub use error::NeuralError;
pub use model::{CharLm, PrefixCache};
pub use p2c::{Candidate, P2c};
pub use qjm::find_model;
pub use scorer::CharScorer;
pub use vocab::Vocab;
