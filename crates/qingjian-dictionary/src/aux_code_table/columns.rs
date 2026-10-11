//! 碼錶檔案裡的列位置：Rime 的 `columns` 列表按名字定下標。

/// Rime `.dict.yaml` 預設的三列，與 Rime 一致。
const DEFAULT_COLUMNS: [&str; 3] = ["text", "code", "weight"];

/// 碼錶檔案裡的列位置：`columns` 列表按名字定下標。表裡沒有 `code` 列就是純詞表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Columns {
    /// 詞在哪一列。
    pub text: usize,

    /// 碼在哪一列；`None` 表示這是純詞表（`columns: [text, weight]`）。
    pub code: Option<usize>,
}

impl Default for Columns {
    fn default() -> Self {
        names_to_columns(&default_column_names())
    }
}

/// 預設列名（`text` / `code` / `weight`）。
pub(super) fn default_column_names() -> Vec<String> {
    DEFAULT_COLUMNS.map(str::to_owned).to_vec()
}

/// 按 `columns` 列表裡的名字定下標；列表裡沒有的名字就是 `None`。`text` 預設在第 0 列。
pub(super) fn names_to_columns(names: &[String]) -> Columns {
    let index_of = |want: &str| names.iter().position(|name| name == want);
    Columns {
        text: index_of("text").unwrap_or(0),
        code: index_of("code"),
    }
}
