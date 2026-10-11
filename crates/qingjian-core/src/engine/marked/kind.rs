/// preedit（marked text）片段的種類，殼按它選樣式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkedKind {
    /// 使用者敲的、參與本次候選的拼音（已按音節用 `'` 切開）。
    Typed,

    /// 游標停在中間時，作用域之後剩下的拼音：只顯示，不參與候選，畫淡一點。
    Rest,

    /// 拼寫糾錯裡被改掉的原字母：畫刪除線，提示使用者我們改了什麼。糾錯功能接入前不會出現。
    Corrected,

    /// 輔碼態的碼段（觸發鍵之後的字母序列）：畫淡色 + 下劃線，與「剩餘拼音畫淡」同一套視覺詞彙。
    AuxCode,
}
