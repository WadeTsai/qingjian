//! 輔碼碼錶：詞 → 碼的對映（`AuxCodeTable` 的 `.qj`），輔碼態逐鍵即篩用；與五筆那類編碼 → 詞的形碼碼錶方向相反。
//!
//! 不復用詞庫儲存的理由見 `docs/design/aux-code.md`。
//! 分節：`TEXT` 詞文本 arena、`CODES` 碼鍵 arena、`ENTR` 條目表（按詞文本位元組序升序，同詞的條目相鄰）、
//! `HASH` 詞 → 條目區間的雜湊索引（槽 = 區間起點，鍵 = 詞文本）。

mod columns;
mod entry;
mod import;
mod imported;
mod info;
mod lookup;
mod parsed;
mod report;
mod table;

#[cfg(test)]
mod tests;

pub use import::import_aux_code_table;
pub use imported::AuxCodeTableImport;
pub use info::{AuxCodeTableInfo, aux_code_table_info};
pub use lookup::AuxCodeLookup;
pub use report::AuxCodeTableImportReport;
pub use table::AuxCodeTable;
