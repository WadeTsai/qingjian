//! 「翻譯選中文字」的進行態。

/// 一次「翻譯選中文字」的進行態。
pub(crate) struct Translation {
    /// 雲端譯文；`None` 表示還在等（候選窗顯示「翻譯中…」）。
    pub(crate) result: Option<String>,
}
