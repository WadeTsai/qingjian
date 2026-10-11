use serde::{Deserialize, Serialize};

use qingjian_core::MarkedKind;

/// preedit 片段的種類，DLL 按它選樣式。是 Core 的 [`MarkedKind`] 的可序列化映象
/// （協議不直接用 Core 的內部列舉，免得兩者耦合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreeditKind {
    /// 使用者敲的、參與本次候選的拼音（已按音節用 `'` 切開）。
    Typed,

    /// 游標停在中間時，作用域之後剩下的拼音：只顯示不參與候選，畫淡一點。
    Rest,

    /// 拼寫糾錯裡被改掉的原字母：畫刪除線。
    Corrected,

    /// 輔碼態的碼段（觸發鍵之後的字母序列）：畫淡色 + 下劃線。
    AuxCode,
}

impl From<MarkedKind> for PreeditKind {
    fn from(kind: MarkedKind) -> Self {
        match kind {
            MarkedKind::Typed => Self::Typed,
            MarkedKind::Rest => Self::Rest,
            MarkedKind::Corrected => Self::Corrected,
            MarkedKind::AuxCode => Self::AuxCode,
        }
    }
}
