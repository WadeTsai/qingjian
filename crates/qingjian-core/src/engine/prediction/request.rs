/// 一次聯想請求：要這段拼音對應的詞，可能還要整句補全。只在組句中發，不在上屏後發。
///
/// 上下文只來自應用給出的游標前後文本；應用給不出就只靠拼音。本地輸入歷史是跨應用拼起來的碎片，不當上下文用。
use super::kind::PredictionKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictionRequest {
    /// 請求序號，單調遞增；回來的結果序號對不上就是過期結果。
    pub sequence: u64,

    /// 組句聯想還是問字。
    pub kind: PredictionKind,

    /// 應用裡游標前的文本，已按觀察視窗裁剪；應用不給時為空。
    pub before: String,

    /// 應用裡游標後的文本，已按觀察視窗裁剪。
    pub after: String,

    /// 正在輸入的拼音（帶切分 `'`），本地切分的結果，拼音有誤時切分也可能是錯的。
    pub pinyin: String,

    /// 使用者實際敲的字母（不含 `'`），給模型糾錯用。
    pub letters: String,

    /// 本地切分的音節數，只是參考。
    pub syllables: usize,

    /// 本地詞庫給出的前幾個候選，幫助模型理解拼音，也讓它別重複第一個。
    pub candidates: Vec<String>,

    /// 本地整句轉換把這段拼音轉成的漢字（問字模式是問題本身的漢字形式），可能有錯字；轉不出為空。
    pub guess: String,

    /// 最多要幾條。
    pub max_items: usize,

    /// 是否還要整句補全。
    pub want_sentence: bool,

    /// 翻譯請求的原文（應用裡選中的文字）；其他種類為空。
    pub text: String,

    /// 翻譯請求的目標語言程式碼（`en` / `ja`，學習語言）；其他種類為空。
    pub target_language: String,
}
