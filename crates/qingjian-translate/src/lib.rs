//! 候選翻譯 annotation：本地 gloss 表查詢。
//!
//! 執行時不聯網。檔案格式為 TSV，一行一個詞，後面每列一條釋義，最多取兩條；
//! 釋義開頭可帶詞性縮寫（`n.` / `v.` / `adj.` …，後跟空格），沒有也行：
//!
//! ```text
//! 詞\t[詞性. ]譯文[\t[詞性. ]譯文]
//! 開發\tv. develop\tn. development
//! 中文\tChinese language
//! ```
//!
//! `#` 開頭為註釋行。每個檔案對應一種學習語言，語言由構造時指定。
//!
//! [`LevelTable`] 是詞彙等級表（`levels-<語言>.tsv`，`詞\t等級`），給詞彙統計按級數詞用。
//! [`PersonalGlossary`] 是使用者目錄裡的個人釋義表（釋義兜底寫入、可手改），[`LayeredTranslator`] 把它疊在隨包表上面。

mod error;
mod glossary;
mod layered_translator;
mod level_table;
mod personal_glossary;

pub use error::GlossaryError;
pub use glossary::Glossary;
pub use layered_translator::LayeredTranslator;
pub use level_table::LevelTable;
pub use personal_glossary::PersonalGlossary;
