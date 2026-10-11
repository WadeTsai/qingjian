use qingjian_core::MarkedKind;

/// preedit 片段的畫法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreeditStyle {
    /// 敲的拼音：正常深淺。
    Typed,

    /// 游標後剩下的拼音：淡一點。
    Rest,

    /// 被糾錯改掉的字母：淡且帶刪除線。
    Struck,
}

impl From<MarkedKind> for PreeditStyle {
    fn from(kind: MarkedKind) -> Self {
        match kind {
            MarkedKind::Typed => Self::Typed,
            MarkedKind::Rest => Self::Rest,
            MarkedKind::Corrected => Self::Struck,
            // macOS 殼還沒接輔碼，先借用剩餘拼音的淡色
            MarkedKind::AuxCode => Self::Rest,
        }
    }
}
