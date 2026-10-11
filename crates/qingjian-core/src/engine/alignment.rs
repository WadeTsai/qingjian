/// 候選的音節逐個對到輸入上的結果：上屏時按它消耗拼音、記個人敲錯表。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Alignment {
    /// 消耗的輸入位元組數（含跳過的 `'`）。
    pub consumed: usize,

    /// 靠敲錯變體對上的音節：(敲的那段字母, 候選的音節)。模糊音對上的不算。
    pub typos: Vec<(String, String)>,
}
