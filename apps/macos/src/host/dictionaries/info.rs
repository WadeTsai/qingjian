use std::path::PathBuf;

/// 偏好設定「詞庫」頁裡的一行：一本隨包領域詞庫，或使用者目錄 `dicts/` 下的一本匯入詞庫。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryInfo {
    /// 檔名主幹，配置 `[dictionaries]` 的 `domains` / `disabled` 裡用它。
    pub stem: String,

    /// 隨包的領域詞庫：開關記在 `domains`，不能移除。
    pub builtin: bool,

    /// 檔案路徑。
    pub path: PathBuf,

    /// 顯示名（`.qj` 的 META 名稱，否則檔名主幹）。
    pub name: String,

    /// 詞條數；檔案壞了是 0。
    pub entries: usize,

    /// 許可證（META 裡的），沒有為空。
    pub license: String,

    /// 是否啟用。
    pub enabled: bool,

    /// 檔案讀不出來（壞檔案）。
    pub broken: bool,
}
