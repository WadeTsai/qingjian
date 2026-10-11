//! 輸入狀態：主題用它顯示「中 / 英」「簡 / 繁」這類指示。顯示什麼字由主題定（`when: "mode.english"` 分兩個文位元組點），
//! 渲染器只給事實；方案名由殼給好顯示名，主題用 `"bind": "mode.scheme"` 顯示。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mode {
    /// 英文模式（`false` 為中文）。
    pub english: bool,

    /// 繁體輸出。
    pub traditional: bool,

    /// 當前模式下標點是全形。
    pub full_width: bool,

    /// 輸入方案的顯示名，如「全拼」「小鶴雙拼」「五筆（86） + 全拼」。
    pub scheme: String,
}
