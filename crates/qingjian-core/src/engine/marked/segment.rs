use super::kind::MarkedKind;

/// preedit（marked text）的一段。整段 preedit 是若干段按順序拼起來，游標位置按拼接後的字元數算。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedSegment {
    /// 文本。
    pub text: String,

    /// 種類，決定樣式。
    pub kind: MarkedKind,
}

impl MarkedSegment {
    pub fn new(text: impl Into<String>, kind: MarkedKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}
