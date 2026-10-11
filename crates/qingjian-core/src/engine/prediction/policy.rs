/// 聯想策略：觀察視窗、條數上限、哪些聯想開著。由 [`super::Predictor`] 提供，Engine 據此裁剪上下文、決定發不發。
///
/// 無論殼傳來多長的文本，Core 都只發這麼多：這是隱私上限的最後一道閘。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredictionPolicy {
    /// 游標前最多看幾個字元。
    pub before: usize,

    /// 游標後最多看幾個字元。
    pub after: usize,

    /// 雲端詞最多補進候選視窗第一頁末尾幾格；0 表示不要雲端詞（整句補全照發）。
    pub slots: usize,

    /// 向雲端最多要幾條（比槽位多要幾條，與本地候選重複的去掉後還能填滿）。
    pub max_items: usize,

    /// 組句中要不要整句補全。
    pub sentence: bool,
}

impl Default for PredictionPolicy {
    fn default() -> Self {
        Self {
            before: 64,
            after: 32,
            slots: 2,
            max_items: 4,
            sentence: true,
        }
    }
}
