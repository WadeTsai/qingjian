use std::path::PathBuf;

/// 一次匯入的結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    /// 寫出的 `.qj`。
    pub path: PathBuf,

    /// 詞庫名（Rime 頭裡的 `name:`，否則檔名主幹）。
    pub name: String,

    /// 詞條數。
    pub entries: usize,
}
