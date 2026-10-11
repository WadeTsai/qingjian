//! 候選詞資料模型。
//!
//! 翻譯是候選詞的 annotation：可選、單語言、最多 [`Translation::MAX_SENSES`] 條釋義。
//! 不要把它擴充套件成多語言並列的結構，那會破壞「一次只學一種語言」的產品原則。

mod furigana;
mod kind;
mod language;
mod layout;
mod list;
mod part_of_speech;
mod sense;
mod translation;

use serde::{Deserialize, Serialize};

pub use furigana::{FuriganaSegment, furigana};
pub use kind::CandidateKind;
pub use language::{Language, UnknownLanguage};
pub use layout::{CandidateLayout, Cell, GRID_ROWS, Grid, MAX_CELL_EMS};
pub use list::CandidateList;
pub use part_of_speech::{PartOfSpeech, UnknownPartOfSpeech};
pub use sense::Sense;
pub use translation::Translation;

/// 一個可上屏的候選。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// 上屏文本。
    pub text: String,

    /// 來源型別。
    pub kind: CandidateKind,

    /// 該候選對應的拼音音節，供平臺層高亮已匹配部分。
    pub syllables: Vec<String>,

    /// 讀音（如日語假名），中文候選暫不使用。
    pub reading: Option<String>,

    /// 學習語言下的譯文；查不到或尚未就緒時為 `None`。
    pub translation: Option<Translation>,

    /// 候選帶的輔碼：篩碼時是命中當前碼段的那條，沒在篩碼（純拼音態、輔碼態空碼段）時是詞的
    /// 首條碼（一詞多碼、多張碼錶取第一條）；沒裝碼錶或這個詞沒有碼時為 `None`。
    /// 殼按 `[general] aux_code_show` 決定要不要顯示。
    #[serde(default)]
    pub aux_code: Option<String>,
}
