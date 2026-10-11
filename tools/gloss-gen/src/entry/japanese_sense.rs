use serde::{Deserialize, Serialize};

/// 一個日文譯詞及其假名讀音。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JapaneseSense {
    /// 寫法（漢字假名混寫）。
    pub text: String,

    /// 全假名讀音；模型給的不是純假名時丟棄。
    pub reading: Option<String>,
}
