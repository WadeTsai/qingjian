//! 釋義兜底：隨包釋義表查不到的詞，上屏後交給 [`GlossFiller`] 在後臺問雲端寫一條釋義，
//! 結果經 [`super::Translator::learn`] 進個人釋義表，下次候選裡就有譯詞。
//!
//! 與聯想（`Predictor`）分開走：聯想是「最新請求優先」、防抖會丟舊請求，兜底恰恰要「每個詞都問到、慢點沒關係」。
//! 介面同樣非阻塞：`request` 只入隊，結果由殼定時 `Engine::poll_glosses` 取。只在雲聯想開著時才有實現接進來
//! （發出去的只有那個詞本身），預設 [`NoGlossFiller`] 什麼都不問。

mod filled_gloss;
mod filler;

pub use filled_gloss::FilledGloss;
pub use filler::{GlossFiller, NoGlossFiller};
