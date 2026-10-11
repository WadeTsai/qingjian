use serde::{Deserialize, Serialize};

use qingjian_core::MarkedSegment;

use super::kind::PreeditKind;

/// preedit（組句拼音行）的一段。整段 preedit 是若干段按順序拼起來，游標位置按拼接後的字元數算。
/// 是 Core 的 [`MarkedSegment`] 的可序列化映象。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreeditSegment {
    /// 文本。
    pub text: String,

    /// 種類，決定樣式。
    pub kind: PreeditKind,
}

impl From<&MarkedSegment> for PreeditSegment {
    fn from(segment: &MarkedSegment) -> Self {
        Self {
            text: segment.text.clone(),
            kind: segment.kind.into(),
        }
    }
}
