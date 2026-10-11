//! preedit 片段的畫法。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreeditStyle {
    /// 敲的拼音：正常深淺。
    Typed,

    /// 游標後剩下的拼音：淡一點。
    Rest,

    /// 被糾錯改掉的字母：淡且帶刪除線。
    Struck,

    /// 輔碼碼段：與剩餘拼音同一個淡色，再壓一道下劃線區分。
    AuxCode,
}
