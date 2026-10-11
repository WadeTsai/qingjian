//! 碼錶頁要顯示的後設資料，讀不動也不報錯。

use std::path::Path;

use super::table::AuxCodeTable;

/// 碼錶頁要顯示的後設資料：名稱、條數、許可、是不是壞檔案。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxCodeTableInfo {
    /// 給人看的名稱（`.qj` 後設資料裡的 name，取不到時是檔名主幹）。
    pub name: String,

    /// 詞條數（詞 → 碼 的條目數）。
    pub entries: usize,

    /// 許可證標識（SPDX）。
    pub license: String,

    /// 開啟失敗（不是碼錶、檔案損壞）時為真，介面標紅。
    pub broken: bool,
}

/// 讀碼錶的後設資料。打不開或不是碼錶時不報錯：返回 `broken = true`，名稱退回檔名主幹，
/// 設定頁照樣把它列出來（使用者能看見並刪掉這個壞檔案）。
pub fn aux_code_table_info(path: &Path) -> AuxCodeTableInfo {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned();
    match AuxCodeTable::open(path) {
        Ok(table) => {
            let metadata = table.metadata();
            AuxCodeTableInfo {
                name: metadata
                    .map(|m| m.name.clone())
                    .filter(|name| !name.is_empty())
                    .unwrap_or(stem),
                entries: table.len(),
                license: metadata.map_or_else(String::new, |m| m.license.clone()),
                broken: false,
            }
        }
        Err(error) => {
            tracing::warn!(file = %path.display(), %error, "码表打不开");
            AuxCodeTableInfo {
                name: stem,
                entries: 0,
                license: String::new(),
                broken: true,
            }
        }
    }
}
