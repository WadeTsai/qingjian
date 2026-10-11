use super::VocabularySummary;
use crate::candidate::Language;

/// 詞彙記錄的落盤方。實現放學習 crate，Core 只查次數、送事件。
pub trait VocabularyTracker: Send {
    /// 譯詞 `word`（學習語言 `language`）在候選窗口裡被看到過幾輪（一輪組句最多記一次）。
    fn exposures(&self, language: Language, word: &str) -> u32;

    /// 使用者上屏時這條譯詞在螢幕上：記一次看到。
    fn record_exposure(&mut self, language: Language, word: &str);

    /// 使用者上屏了帶這條譯詞的候選：`used` 是直接把譯詞打出去了（修飾鍵 + 數字），否則只是上屏了中文、看著譯詞。
    fn record_commit(&mut self, language: Language, word: &str, used: bool);

    /// 落盤。殼在停用輸入法時呼叫，啟用期間也定時調；失敗只記日誌。
    fn flush(&mut self) {}

    /// 某種學習語言的彙總（偏好設定「統計」頁）。
    fn summary(&self, _language: Language) -> VocabularySummary {
        VocabularySummary::default()
    }
}

/// 不記詞彙：所有譯詞都算看熟了，候選裡不標生詞。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoVocabularyTracker;

impl VocabularyTracker for NoVocabularyTracker {
    fn exposures(&self, _language: Language, _word: &str) -> u32 {
        u32::MAX
    }

    fn record_exposure(&mut self, _language: Language, _word: &str) {}

    fn record_commit(&mut self, _language: Language, _word: &str, _used: bool) {}
}
