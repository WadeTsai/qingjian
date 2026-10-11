/// 詞圖一個格子裡的一個詞：從詞庫命中裡拷出來、不再借用詞庫的形式，能放進 [`super::SpanCache`]。
#[derive(Debug, Clone, PartialEq)]
pub struct SpanWord {
    /// 詞。
    pub text: String,

    /// 詞的音節。
    pub syllables: Vec<String>,

    /// 詞庫靜態詞頻。
    pub frequency: u32,

    /// 命中的音節不是敲的原樣（模糊音 / 敲錯變體）時的代價之和，進路徑得分時扣掉；原樣命中是 0。
    pub penalty: f64,
}
