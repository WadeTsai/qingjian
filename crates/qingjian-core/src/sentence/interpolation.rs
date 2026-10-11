use super::{CONFIDENCE_K, MAX_CONFIDENCE, TRIGRAM_DISCOUNT, USER_LAMBDA};

/// 個人 n-gram 與靜態模型插值的引數（見 [`UserNgram::blend`]）。預設值是 `sentence` 模組裡的常數，
/// 回放調參（`qingjian-cli --tune`）時可以整組換掉，引擎與殼只用預設值。
///
/// [`UserNgram::blend`]: super::UserNgram::blend
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interpolation {
    /// 個人機率裡 bigram 部分的權重，其餘給個人一元（[`USER_LAMBDA`]）。
    pub lambda: f64,

    /// 插值權重 μ = c(v)/(c(v)+K) 裡的 K（[`CONFIDENCE_K`]）。
    pub confidence_k: f64,

    /// 插值權重的封頂（[`MAX_CONFIDENCE`]）。
    pub max_confidence: f64,

    /// 個人三元的絕對摺扣 D（[`TRIGRAM_DISCOUNT`]）。
    pub trigram_discount: f64,
}

impl Interpolation {
    /// 現在的常數。
    pub const DEFAULT: Self = Self {
        lambda: USER_LAMBDA,
        confidence_k: CONFIDENCE_K,
        max_confidence: MAX_CONFIDENCE,
        trigram_discount: TRIGRAM_DISCOUNT,
    };
}

impl Default for Interpolation {
    fn default() -> Self {
        Self::DEFAULT
    }
}
