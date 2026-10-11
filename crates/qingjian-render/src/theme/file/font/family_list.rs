//! 字族回退鏈：`"Songti SC"` 或 `["Songti SC", "STSong", "system"]`，按順序用第一個裝了的；`system` 是介面字型（使用者在設定裡選的字型，沒選就是系統字型）。

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "Raw")]
pub(crate) struct FamilyList(pub(crate) Vec<String>);

#[derive(Deserialize)]
#[serde(untagged)]
enum Raw {
    One(String),

    Many(Vec<String>),
}

impl From<Raw> for FamilyList {
    fn from(raw: Raw) -> Self {
        Self(match raw {
            Raw::One(name) => vec![name],
            Raw::Many(names) => names,
        })
    }
}

impl FamilyList {
    /// 鏈裡代表介面字型的名字。
    pub(crate) const SYSTEM: &str = "system";

    /// 只有介面字型。
    pub(crate) fn system() -> Self {
        Self(vec![Self::SYSTEM.to_owned()])
    }

    /// 是不是介面字型這個名字。
    pub(crate) fn is_system(name: &str) -> bool {
        name.eq_ignore_ascii_case(Self::SYSTEM)
    }
}
