/// 一個詞的上文：前一個詞與再前一個詞。整句路徑上的每一步、詞級排序、個人 n-gram 的記錄與打分都用它。
///
/// 句首詞兩個都是 `None`；句子第二個詞 `previous` 有值、`earlier` 為 `None`（前詞在句首）。
/// 靜態模型只看 `previous`（bigram），個人 n-gram 兩個都看（trigram）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Context<'a> {
    /// 前一個詞；`None` 表示這個詞在句首。
    pub previous: Option<&'a str>,

    /// 前一個詞之前的那個詞；`None` 表示前一個詞在句首（或本身沒有前詞）。
    pub earlier: Option<&'a str>,
}

impl<'a> Context<'a> {
    /// 句首。
    pub const START: Self = Self {
        previous: None,
        earlier: None,
    };

    /// 只知道前一個詞，它在句首。
    pub fn after(previous: &'a str) -> Self {
        Self {
            previous: Some(previous),
            earlier: None,
        }
    }

    /// 前兩個詞都知道：`earlier` 在前、`previous` 緊挨著這個詞。
    pub fn after_two(earlier: &'a str, previous: &'a str) -> Self {
        Self {
            previous: Some(previous),
            earlier: Some(earlier),
        }
    }
}
