/// 日文譯詞的一段：一段漢字帶它的假名，或一段原樣的假名 / 符號。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuriganaSegment {
    /// 這一段的寫法。
    pub text: String,

    /// 這一段是漢字時的平假名讀音；假名段沒有。
    pub reading: Option<String>,
}
