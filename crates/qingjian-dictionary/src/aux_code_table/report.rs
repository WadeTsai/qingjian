//! 一次碼錶匯入的統計口徑。

/// 一次碼錶匯入的統計。與詞庫匯入不同，「有詞無碼」不靜默跳過：碼錶少一行就是少一個詞可用。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuxCodeTableImportReport {
    /// 讀入的正文行數（不含註釋、空行與前導的 YAML 頭）。
    pub read: usize,

    /// 成功建成條目的行數。
    pub with_code: usize,

    /// 有詞但沒有碼的行數（碼列為空，或表裡根本沒有碼列）。
    pub no_code: usize,

    /// 碼非法（不是 a-z、或長度不在 1–8）被丟掉的行數。
    pub skipped: usize,
}
