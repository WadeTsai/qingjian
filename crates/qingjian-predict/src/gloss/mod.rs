//! 釋義兜底：[`qingjian_core::GlossFiller`] 的網路實現。
//!
//! 獨立執行緒、獨立通道，與聯想互不干擾：把主執行緒送來的詞攢成一小批（等 [`worker::BATCH_WAIT`] 或攢夠
//! [`worker::BATCH_SIZE`]），一次請求讓模型給每個詞寫 1–2 條學習語言的譯詞（提示詞與 `gloss-gen` 同源，
//! 只要當前學習語言），解析、清洗後逐條回給主執行緒。問過的詞本程序內不再問，失敗也不重試（下次上屏再說）。

mod cloud_gloss_filler;
mod prompt;
mod worker;

pub use cloud_gloss_filler::CloudGlossFiller;
