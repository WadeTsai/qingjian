use serde::{Deserialize, Serialize};

/// 配置檔案 `[model]` 分節：本地整句模型的開關。
///
/// 隨包的字級小模型在本機給整句候選重新排序，全程離線、不聯網，與雲聯想互不影響（本地先出、雲端到了另佔它自己的格）。
/// 模型檔案不在包裡（或使用者目錄 `model/` 裡）時開關無效。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModelConfig {
    /// 開著就載入模型、給整句重排。
    pub enabled: bool,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}
