//! 輸入統計：每次上屏折成幾個漢字、幾個中文詞、幾個英文詞，交給 [`UsageMeter`] 按天累計。
//!
//! 這是給使用者看的「用青簡打了多少字」，不是學習資料，也不從輸入日誌算（日誌可以關、可以清，統計還在）。
//! Core 只負責算數（[`Usage::of_text`] 與 `Engine` 裡的詞數），按天累計與落盤由實現做
//! （`qingjian-learning` 的 `UsageStats`）；預設 [`NoUsageMeter`] 什麼都不記。

mod book;
mod meter;
mod summary;
mod usage;

pub use book::{BOOKS, Book, book_scale};
pub use meter::{NoUsageMeter, UsageMeter};
pub use summary::UsageSummary;
pub use usage::Usage;
