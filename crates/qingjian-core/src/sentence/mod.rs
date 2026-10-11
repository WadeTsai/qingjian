//! 離線整句轉換：把一串音節轉成最可能的詞序列（`woxiangqu` → 我想去）。
//!
//! 詞圖上每個格子放正好覆蓋那幾個音節的詞（每格只留詞頻最高的幾個），Viterbi + 束搜尋找最優路徑。
//! 打分來自 [`LanguageModel`]（bigram，`qingjian-lm` 實現），模型不認識的詞用詞庫詞頻（一元）兜底並扣分；
//! 使用者選過的詞（Learner 的 weight）加分。沒接模型時整體退化為一元詞頻。
//!
//! 個人 n-gram（[`UserNgram`]，二元 + 三元，隨上屏線上更新、由 Learner 持有）與靜態模型插值，讓整句越用越像自己：
//! 靜態模型只看前一個詞，個人部分看前兩個詞（[`Context`]），Viterbi 裡前二詞取最優前驅的回指，不擴狀態。
//!
//! 簡拼位置（`wxq` 的 `w x q`）按字首取詞，每個格子多留一些候選，全靠語言模型在路徑上分辨；
//! 全拼句子末尾沒打完的單字母不參與，簡拼句子裡末尾單字母就是一個音節。
//!
//! 格子查詞是最貴的一步，結果放進 [`SpanCache`]：敲鍵是增量的，每一鍵只有以它結尾的幾個格子是新的。

mod context;
mod conversion;
mod interpolation;
mod language_model;
mod personal;
mod scorer;
mod sentence_word;
mod span;
mod text_segment;
mod user_ngram;
mod viterbi;

pub use context::Context;
pub use conversion::Conversion;
pub use interpolation::Interpolation;
pub use language_model::{LanguageModel, NoLanguageModel};
pub use personal::Personal;
pub use scorer::SentenceScorer;
pub use sentence_word::SentenceWord;
pub use span::{MAX_SPAN_CACHE_ENTRIES, SpanCache, SpanWord};
pub(crate) use text_segment::is_han;
pub use text_segment::{MAX_TEXT_WORD_CHARS, segment_text};
pub use user_ngram::UserNgram;
pub use viterbi::{convert, convert_paths, convert_whole, convert_with};

/// 句首標記：個人 n-gram 裡句首詞的前詞。與 `qingjian-lm` 語料統計用的是同一個記號。
pub const SENTENCE_START: &str = "<s>";

/// 個人機率裡 bigram 部分的權重，其餘給個人一元。
pub const USER_LAMBDA: f64 = 0.8;

/// 個人插值權重 μ = c(v)/(c(v)+K)：前詞見過 K 次時個人資料與靜態模型各佔一半（再受封頂限制）。
/// 取 8 是為了一次誤選翻不過靜態 bigram，選兩次才翻。
pub const CONFIDENCE_K: f64 = 8.0;

/// 個人插值權重的封頂：使用者資料再多，沒見過的接續也最多打這個折，保留說新話的餘地。
pub const MAX_CONFIDENCE: f64 = 0.5;

/// 個人三元的絕對摺扣 D：每條見過的三元接續讓出 D 份機率給二元回退，見得少的上文回退得多。
/// 0.75 是 n-gram 平滑的慣用值，個人資料量小、一次見過的接續佔多數時它決定了三元能壓過二元多少。
pub const TRIGRAM_DISCOUNT: f64 = 0.75;

/// 個人 n-gram 最多存多少條轉移（二元對 + 三元條），超過就整體減半（忘掉久遠的偏好）。一年的個人輸入遠到不了這個量。
pub const MAX_USER_TRANSITIONS: usize = 200_000;

/// 一個詞最多幾個音節；更長的詞庫裡有但極少，限制它讓詞圖規模可控。
pub const MAX_WORD_SYLLABLES: usize = 8;

/// 末尾未打完的音節至少要幾個字母才參與整句：單個字母的字首範圍太大（`s` 匹配所有 s 開頭的音節），不值得掃。
pub const MIN_PARTIAL_LETTERS: usize = 2;

/// 每個格子最多留幾個詞（按詞庫詞頻 + 使用者加分）。同音詞很多，全留會讓束搜尋白費。
pub const SPAN_CANDIDATES: usize = 6;

/// 有簡拼位置的格子最多留幾個詞：`h` 下有 和 / 好 / 會 / 還 / 很 …… 幾十個常用字，
/// 只留六個會把句子裡要的那個擠掉，多留一些讓語言模型去挑。
pub const ABBREVIATED_SPAN_CANDIDATES: usize = 20;

/// 每個位置最多保留幾條部分路徑。
pub const BEAM_WIDTH: usize = 8;

/// 語言模型不認識、只能按詞庫詞頻兜底的詞扣多少分：模型見過的詞更可信。
pub const FALLBACK_PENALTY: f64 = -4.0;

/// 模型不認識的詞的兜底 log 機率：詞庫詞頻佔總詞頻的比例再扣 [`FALLBACK_PENALTY`]。`log_total` 是總詞頻的對數。
pub fn fallback_log_prob(frequency: u32, log_total: f64) -> f64 {
    (f64::from(frequency) + 1.0).ln() - log_total + FALLBACK_PENALTY
}

/// `log P(word | context)`：先問靜態模型（只看前一個詞，不認識就用 `fallback`），再與個人 n-gram（看前兩個詞）插值。
/// 整句路徑上的每一步和詞級排序的上下文得分都用它。
pub fn transition_log_prob(
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    context: Context<'_>,
    word: &str,
    fallback: f64,
) -> f64 {
    let base = model.log_prob(context.previous, word).unwrap_or(fallback);
    personal.blend(context, word, base)
}
