//! 點中的是什麼：第幾個候選，或第幾個候選的第幾條譯詞。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    /// 這一頁第 `n` 個候選（從 0 起）。
    Candidate(usize),

    /// 這一頁第 `candidate` 個候選右側的第 `sense` 條譯詞（從 0 起，與「上屏第一 / 第二條譯詞」的鍵對應）。
    Translation { candidate: usize, sense: usize },
}
