//! annotation 片段的種類：決定顏色，主題也按它挑著畫（`annotation` 節點的 `tones`）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 譯文。
    Gloss,

    /// 生詞的譯文（使用者還沒在候選裡見過幾輪），用強調色。
    Fresh,

    /// 其餘的淡色片段（日文譯詞的假名注音），最淺。
    Faint,

    /// 詞性（`n.`、`int.`），預設與 `Faint` 同色。
    Pos,

    /// 義項之間的分隔（` · `），預設與 `Faint` 同色；主題按它數義項。
    Separator,

    /// 輔碼態命中的那條碼（`[general] aux_code_show` 開啟時才有）：與譯文同一個淡色。
    Code,
}
