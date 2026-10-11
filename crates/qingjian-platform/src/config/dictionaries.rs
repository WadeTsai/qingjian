use serde::{Deserialize, Serialize};

/// 配置檔案 `[dictionaries]` 分節：附加詞庫的開關。
///
/// 兩類附加詞庫：隨包的領域詞庫（`.app` 裡 `Resources/dicts/`，法律 / 醫學 / 地名 …）預設關閉，列在 `domains` 裡的才載入；
/// 使用者目錄 `dicts/` 下的 `.qj`（自己匯入的）檔案在就載入，只有列在 `disabled` 裡的（按檔名，不含副檔名）跳過；
/// 匯入 / 移除就是加 / 刪檔案。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DictionariesConfig {
    /// 開啟的隨包領域詞庫（檔名，不含 `.qj`）。
    pub domains: Vec<String>,

    /// 關掉的使用者詞庫（檔名，不含 `.qj`）。
    pub disabled: Vec<String>,
}

/// 預設開啟的隨包領域詞庫：成語四字全拼幾乎不歧義，收益穩；其餘按需開啟。
pub const DEFAULT_DOMAINS: [&str; 1] = ["idioms"];

impl Default for DictionariesConfig {
    fn default() -> Self {
        Self {
            domains: DEFAULT_DOMAINS.iter().map(|s| (*s).to_owned()).collect(),
            disabled: Vec::new(),
        }
    }
}

impl DictionariesConfig {
    /// 使用者目錄裡的詞庫是否啟用。
    pub fn is_enabled(&self, stem: &str) -> bool {
        !self.disabled.iter().any(|d| d == stem)
    }

    /// 隨包領域詞庫是否啟用。
    pub fn is_domain_enabled(&self, stem: &str) -> bool {
        self.domains.iter().any(|d| d == stem)
    }
}
