use crate::sentence::Context;

/// 連續上屏的鏈：記住最近上屏的兩個中文詞，下一個詞上屏時就能記一條帶前二詞的轉移（個人 n-gram 的三元）。
///
/// 標點、透傳、回車上屏拼音、英文詞等都打斷鏈（下一個詞按句首記）。
#[derive(Debug, Default, Clone)]
pub struct CommitChain {
    /// 上一個上屏的中文詞及其音節；`None` 表示下一個詞在句首。
    previous: Option<(String, Vec<String>)>,

    /// 上一個詞之前的那個詞；`None` 表示上一個詞在句首。
    earlier: Option<String>,

    /// 上一個詞上屏後緩衝區裡還留著拼音：下一個詞若緊接著從同一段拼音裡選出，兩個詞本來是一起打的。
    same_buffer: bool,

    /// 當前這段拼音裡已經上屏的詞（文本與音節），整段選完時看要不要合成一個詞。
    buffer_words: Vec<(String, Vec<String>)>,

    /// 這段拼音整段的學習鍵（按輸入串記選擇用的全部字母），第一個詞上屏前記下。
    buffer_key: String,
}

impl CommitChain {
    /// 上一個詞（若有）。
    pub fn previous(&self) -> Option<&str> {
        self.previous.as_ref().map(|(text, _)| text.as_str())
    }

    /// 上一個詞的音節。
    pub fn previous_syllables(&self) -> &[String] {
        self.previous.as_ref().map_or(&[], |(_, s)| s.as_slice())
    }

    /// 下一個詞的上文：前一個詞與再前一個詞。
    pub fn context(&self) -> Context<'_> {
        Context {
            previous: self.previous(),
            earlier: self.earlier.as_deref(),
        }
    }

    /// 鏈上是否有這個詞（作為前一個或再前一個）。
    pub fn mentions(&self, text: &str) -> bool {
        self.previous() == Some(text) || self.earlier.as_deref() == Some(text)
    }

    /// 下一個詞是否與上一個詞出自同一段拼音。
    pub fn same_buffer(&self) -> bool {
        self.same_buffer
    }

    /// 記下剛上屏的詞；`buffer_left` 是上屏後緩衝區裡是否還有拼音。
    pub fn advance(&mut self, text: &str, syllables: &[String], buffer_left: bool) {
        self.earlier = self.previous.take().map(|(text, _)| text);
        self.previous = Some((text.to_owned(), syllables.to_vec()));
        self.same_buffer = buffer_left;
        self.buffer_words
            .push((text.to_owned(), syllables.to_vec()));
    }

    /// 一段新拼音裡的第一個詞要上屏了：記下整段的學習鍵，上一段的詞清掉。
    pub fn begin_buffer(&mut self, key: String) {
        self.buffer_words.clear();
        self.buffer_key = key;
    }

    /// 當前這段拼音裡已上屏的詞，按順序。
    pub fn buffer_words(&self) -> &[(String, Vec<String>)] {
        &self.buffer_words
    }

    /// 當前這段拼音整段的學習鍵。
    pub fn buffer_key(&self) -> &str {
        &self.buffer_key
    }

    /// 打斷鏈。
    pub fn reset(&mut self) {
        self.previous = None;
        self.earlier = None;
        self.same_buffer = false;
        self.buffer_words.clear();
    }

    /// 緩衝區被清空或整段被別的東西吃掉：鏈不斷，但下一個詞不算同一段拼音。
    pub fn leave_buffer(&mut self) {
        self.same_buffer = false;
        self.buffer_words.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_last_two_words_as_context() {
        let mut chain = CommitChain::default();
        assert_eq!(chain.context(), Context::START);
        chain.advance("我", &["wo".into()], true);
        assert_eq!(chain.context(), Context::after("我"));
        chain.advance("想", &["xiang".into()], false);
        assert_eq!(chain.context(), Context::after_two("我", "想"));
        chain.advance("去", &["qu".into()], false);
        assert_eq!(chain.context(), Context::after_two("想", "去"));
        assert!(chain.mentions("想") && chain.mentions("去") && !chain.mentions("我"));
        chain.reset();
        assert_eq!(chain.context(), Context::START);
    }
}
