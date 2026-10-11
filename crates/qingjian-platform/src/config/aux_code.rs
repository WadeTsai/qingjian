//! 配置檔案 `[aux_code]` 分節：輔碼碼錶的開關。

use serde::{Deserialize, Serialize};

/// 配置檔案 `[aux_code]` 分節：輔碼碼錶的開關。
///
/// 與 `[dictionaries]` 同形：使用者目錄 `codes/` 下的 `.qj` 檔案在就載入，只有列在 `disabled` 裡的
/// （按檔名，不含副檔名）跳過；匯入 / 移除就是加 / 刪檔案。
/// `enabled` 是整條輔碼線的總開關（預設關）：關著時 `;` 完全保持原生行為，開著但沒有任何
/// 可用碼錶也不進輔碼態。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AuxCodeConfig {
    /// 輔碼總開關，預設關。關著時觸發鍵 `;` 完全保持原生行為（全形標點 / 雙拼韻母鍵）。
    pub enabled: bool,

    /// 關掉的碼錶（檔名，不含 `.qj`）。
    pub disabled: Vec<String>,
}

impl AuxCodeConfig {
    /// 使用者目錄裡的碼錶是否啟用。
    pub fn is_enabled(&self, stem: &str) -> bool {
        !self.disabled.iter().any(|d| d == stem)
    }
}
