/// 使用者要求刪掉一個候選（[`super::Engine::forget`]）之後，實際清掉了什麼。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Forgotten {
    /// 它是使用者詞（雲端詞、自動造詞、匯入前學的詞），已從使用者詞裡刪掉，本地不再出。
    pub user_word: bool,

    /// 清掉了對它的學習：選擇次數、按輸入串記的選擇、個人 n-gram 裡與它有關的轉移。
    pub learning: bool,
}

impl Forgotten {
    /// 什麼都沒清（詞庫裡的詞，也沒學過）。
    pub fn is_nothing(&self) -> bool {
        !self.user_word && !self.learning
    }
}
