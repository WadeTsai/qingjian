//! 一次碼錶匯入的結果。

use std::path::PathBuf;

use super::report::AuxCodeTableImportReport;

/// 一次碼錶匯入的結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxCodeTableImport {
    /// 寫出的 `.qj`。
    pub path: PathBuf,

    /// 碼錶名（Rime 頭裡的 `name:`，否則檔名主幹）。
    pub name: String,

    /// 匯入統計。
    pub report: AuxCodeTableImportReport,
}
