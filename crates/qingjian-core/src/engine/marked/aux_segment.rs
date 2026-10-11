//! 輔碼態的 preedit 分段形態（觸發鍵與碼段）。

/// 輔碼態在 preedit 裡的兩段：觸發鍵按 `Typed` 畫（不突出），碼段是 `MarkedKind::AuxCode` 段。
///
/// 觸發鍵只顯示、不進緩衝區，也不參與候選；碼段與拼音段分開記賬（見 `Engine` 的 `aux_code` 欄位）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxSegment {
    /// 觸發鍵（預設 `;`）。使用者敲的是它，但緩衝區裡沒有它。
    pub trigger: char,

    /// 碼段：只 `a-z`，無游標概念、只從末尾增刪。剛觸發、還沒敲碼時是空串。
    pub code: String,
}

impl AuxSegment {
    pub fn new(trigger: char, code: String) -> Self {
        Self { trigger, code }
    }
}
