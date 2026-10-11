use super::Transition;

/// 最近一次上屏記了哪些學習：使用者把它退格刪掉、再打同一段拼音換選別的詞時，把這些記錄退回去。
/// 沒記學習的上屏（英文詞、標點、原樣上屏）也留一條只有長度的記錄，退格數過它才能數到更早的詞。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastCommit {
    /// 上屏的文本。
    pub text: String,

    /// 文本的字元數：退格這麼多次就算整個刪掉了。
    pub chars: usize,

    /// 消耗掉的那段拼音（按輸入串記選擇用的鍵）。
    pub input: String,

    /// 記過一次選擇次數（`Learner::record`）與輸入串選擇（`record_choice`）的詞；整句上屏沒有。
    pub chosen: Option<String>,

    /// 記過的詞轉移（含上文與份數）。
    pub transitions: Vec<Transition>,

    /// 記進個人敲錯表的 (敲的, 要的) 音節對。
    pub typos: Vec<(String, String)>,

    /// 上屏之後已經退格了幾個字元。
    pub erased: usize,

    /// 輸入日誌裡這次上屏的序號，撤銷時指回去。
    pub log_id: u64,

    /// 整段拼音分幾次選完時記的「整段 → 合成詞」選擇（學習鍵, 合成詞），撤銷時一併退回。
    pub phrase: Option<(String, String)>,
}

impl LastCommit {
    /// 沒記任何學習的上屏：只記文本長度。
    pub fn plain(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            chars: text.chars().count(),
            input: String::new(),
            chosen: None,
            transitions: Vec::new(),
            typos: Vec::new(),
            erased: 0,
            log_id: 0,
            phrase: None,
        }
    }

    /// 已經整個刪掉了。
    pub fn is_erased(&self) -> bool {
        self.chars > 0 && self.erased == self.chars
    }

    /// 再打的這段拼音是不是同一段：完全相同，或是它的字首（重打後只選了前面一個短詞）。
    pub fn same_input(&self, input: &str) -> bool {
        !input.is_empty() && self.input.starts_with(input)
    }
}
