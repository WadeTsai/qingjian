//! 詞庫：按拼音音節序列查詞。
//!
//! 純資料層，不依賴任何兄弟 crate。檔案格式為 TSV：
//!
//! ```text
//! 詞\t音節（空格分隔）\t詞頻
//! 開發\tkai fa\t9000
//! ```
//!
//! `#` 開頭為註釋行，空行忽略。
//!
//! 記憶體佈局面向「幾十萬到上百萬條常駐」：詞文本與拼音鍵各放一個連續 arena，
//! 詞目只存偏移與詞頻，鍵排序後二分定位、順序掃描字首範圍。
//!
//! 輔碼碼錶（[`AuxCodeTable`]）是另一套資料：詞 → 碼的對映，按詞文本排序 + 雜湊索引定位（見 `aux_code_table` 模組）。
//! 另有兩種旁支：形碼碼錶 [`CodeTable`]（鍵是五筆編碼，按字首查）與英文詞表 [`WordList`]。

mod aux_code_table;
mod code_table;
mod dictionary;
mod error;
pub mod import;
mod matching;
mod pattern;
mod word_list;

pub use aux_code_table::{
    AuxCodeLookup, AuxCodeTable, AuxCodeTableImport, AuxCodeTableImportReport, AuxCodeTableInfo,
    aux_code_table_info, import_aux_code_table,
};
pub use code_table::CodeTable;
pub use dictionary::Dictionary;
pub use error::DictionaryError;
pub use matching::Match;
pub use pattern::{SyllablePattern, canonical_syllable};
pub use word_list::WordList;
