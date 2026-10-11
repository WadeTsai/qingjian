//! 輸入日誌：每次上屏記一條「敲了什麼、看到了什麼、選了什麼」，退格撤銷、退格重打、直通字元、雲端聯想、
//! 上文斷開與會話資訊各記一條。格式與各事件的用途見 `docs/plan/model-eval.md`。
//!
//! 聚合的學習資料（選擇次數、輸入串選擇、個人 n-gram）回答不了這個問題，而排序 / 整句的離線迴歸評測
//! 與個人模型的訓練對都要它。Core 只產生條目（[`InputLogEntry`]），寫到哪、加不加時間戳由
//! [`InputLogger`] 的實現定（`qingjian-learning` 寫本機 jsonl）；預設 [`NoInputLogger`] 什麼都不記。
//! 條目裡只有使用者敲的鍵與上屏的文字，不含應用裡的上下文。

mod entry;
mod logger;
mod muted;
mod source;

pub use entry::{CommitEntry, INPUT_LOG_VERSION, InputLogEntry};
pub use logger::{InputLogger, NoInputLogger};
pub(super) use muted::MutedLogger;
pub use source::InputSource;

/// 寫進條目的候選文本最多幾條：一頁的量，夠算「首選命中了沒有」「在不在第一頁」和「選的是第幾個」。
pub const LOGGED_CANDIDATES: usize = 9;
