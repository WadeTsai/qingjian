//! 節點上的顏色：一個顏色引用，或按條件二選一 `{ "if": "highlighted", "then": "@a", "else": "@b" }`
//! （條件寫法同 `when`）。高亮的候選換白字這類「變體」就靠它，不用把節點抄兩份。

use serde::Deserialize;

use super::ColorRef;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum ColorSpec {
    /// 固定的顏色。
    Fixed(ColorRef),

    /// 條件成立用 `then`，否則用 `otherwise`。
    Switch {
        #[serde(rename = "if")]
        condition: String,

        then: ColorRef,

        #[serde(rename = "else")]
        otherwise: ColorRef,
    },
}

impl ColorSpec {
    /// 兩個分支的顏色引用（校驗用）。
    pub(crate) fn refs(&self) -> Vec<&ColorRef> {
        match self {
            Self::Fixed(color) => vec![color],
            Self::Switch {
                then, otherwise, ..
            } => vec![then, otherwise],
        }
    }
}
