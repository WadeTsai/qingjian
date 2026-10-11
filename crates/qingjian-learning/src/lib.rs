//! 使用者詞頻學習與輸入日誌的落盤。
//!
//! [`FrequencyLearner`] 存聚合資料（選擇次數、輸入串選擇、使用者詞、個人英文詞、個人 n-gram，都是 TSV）；
//! [`InputLog`] 逐條記上屏（jsonl），給離線迴歸評測與個人模型用；[`UsageStats`] 按天數打了多少字（`usage.tsv`），
//! [`VocabularyBook`] 記學習語言的譯詞看過 / 上屏過幾次（`user-vocab.tsv`，候選裡標生詞的依據）。

mod error;
mod frequency_learner;
mod input_log;
mod usage_stats;
mod vocabulary_book;

pub use error::LearningError;
pub use frequency_learner::FrequencyLearner;
pub use input_log::InputLog;
pub use usage_stats::UsageStats;
pub use vocabulary_book::VocabularyBook;
