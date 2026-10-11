/// 詞彙彙總裡一個等級的數字。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelCount {
    /// 等級名（`A1` / `N5`）。
    pub name: String,

    /// 等級表裡這一級共有多少詞。
    pub total: u64,

    /// 其中見過的。
    pub seen: u64,

    /// 其中看熟了的。
    pub familiar: u64,

    /// 其中上屏過的。
    pub committed: u64,
}
