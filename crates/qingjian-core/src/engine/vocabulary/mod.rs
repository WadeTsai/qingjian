//! 學習語言的詞彙記錄：使用者在候選窗口裡見過哪些譯詞、上屏過哪些、用修飾鍵 + 數字直接打出過哪些。
//!
//! 「生詞」就從這裡判斷：一條譯詞被看到的輪次還不到 [`FRESH_UNTIL`]，候選裡就標出來（[`crate::candidate::Sense::fresh`]），
//! 看熟了自然不標。「看到」按上屏那一刻螢幕上的那一頁算（`Engine::note_displayed` 記下當前頁，上屏時才記進詞彙表），
//! 逐鍵重新整理時一閃而過的候選不算：使用者真正讀候選是在要選的時候。
//! Core 只判斷與記錄，按詞落盤與彙總由實現做（`qingjian-learning` 的 `VocabularyBook`）；預設 [`NoVocabularyTracker`] 不記、也不標生詞。

mod level_count;
mod summary;
mod tracker;

pub use level_count::LevelCount;
pub use summary::VocabularySummary;
pub use tracker::{NoVocabularyTracker, VocabularyTracker};

/// 見過這麼多輪之前算生詞。
pub const FRESH_UNTIL: u32 = 3;
