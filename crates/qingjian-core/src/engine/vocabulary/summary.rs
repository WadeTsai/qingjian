use super::LevelCount;

/// 一種學習語言的詞彙彙總。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VocabularySummary {
    /// 見過的不同譯詞數。
    pub seen: u64,

    /// 其中看熟了的（見過的輪次不少於 [`super::FRESH_UNTIL`]）。
    pub familiar: u64,

    /// 上屏過帶這條譯詞的候選的不同譯詞數。
    pub committed: u64,

    /// 直接打出過的不同譯詞數。
    pub used: u64,

    /// 最近 7 天（含今天）第一次見到的譯詞數。
    pub new_this_week: u64,

    /// 按等級（從易到難）分的數字；沒有等級表時為空。
    pub levels: Vec<LevelCount>,
}
