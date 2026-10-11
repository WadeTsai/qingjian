use crate::sentence::Context;

/// 記進個人 n-gram 的一條轉移：上屏時記下來，使用者緊接著退格刪掉重選時原樣退回。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// 前一個詞之前的那個詞；`None` 表示前一個詞在句首。
    pub earlier: Option<String>,

    /// 前一個詞；`None` 表示這個詞在句首。
    pub previous: Option<String>,

    /// 上屏的詞。
    pub word: String,

    /// 記了幾份（使用者點選的雙份，整句路徑順帶的一份）。
    pub times: u32,
}

impl Transition {
    pub fn new(context: Context<'_>, word: &str, times: u32) -> Self {
        Self {
            earlier: context.earlier.map(str::to_owned),
            previous: context.previous.map(str::to_owned),
            word: word.to_owned(),
            times,
        }
    }

    /// 記錄時的上文。
    pub fn context(&self) -> Context<'_> {
        Context {
            previous: self.previous.as_deref(),
            earlier: self.earlier.as_deref(),
        }
    }
}
