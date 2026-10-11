use super::SentenceWord;

/// 一次整句轉換的結果。
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    /// 整句文本。
    pub text: String,

    /// 覆蓋的全拼音節，與 `text` 逐字對應。
    pub syllables: Vec<String>,

    /// 路徑上的詞，按順序。只有一個說明整段本來就是一個詞，不必當整句顯示。
    pub words: Vec<SentenceWord>,

    /// 路徑得分（log 機率之和），越大越好。
    pub score: f64,

    /// `score` 裡靜態語言模型那部分（每步只問靜態模型、不認識就兜底的 log 機率之和），不含個人 n-gram 插值、
    /// 使用者加分與代價。神經重打分用它：神經分替換的是靜態模型的判斷，個人的那些原樣保留。
    pub static_score: f64,

    /// 路徑上模糊音 / 敲錯變體的代價之和（已含在 `score` 裡）：大於 0 說明這條路徑不是按敲的原樣讀的。
    pub penalty: f64,
}

impl Conversion {
    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    /// 路徑裡有音節不是按敲的原樣讀的（模糊音或敲錯變體）。
    pub fn altered(&self) -> bool {
        self.penalty > 0.0
    }

    /// 路徑裡有佔位音節（沒轉成字的拼音）。
    pub fn has_placeholder(&self) -> bool {
        self.words.iter().any(|w| w.placeholder)
    }
}
