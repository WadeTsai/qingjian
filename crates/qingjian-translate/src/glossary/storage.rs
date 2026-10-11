use std::collections::HashMap;

use qingjian_core::Translation;
use qingjian_format::{Table, Text};

use super::entry_record::EntryRecord;
use super::sense_record::SenseRecord;

/// 釋義表的兩種存法：TSV 解析出來的雜湊表，或對映的 `.qj`（arena + 定長表 + 檔案裡的雜湊索引）。
#[derive(Debug)]
pub enum Storage {
    /// 記憶體裡自己的。
    Owned(HashMap<String, Translation>),

    /// 對映檔案裡的。
    Mapped {
        /// 所有字串首尾相接。
        text: Text,

        /// 詞條表，下標即編號。
        entries: Table<EntryRecord>,

        /// 釋義表。
        senses: Table<SenseRecord>,

        /// 詞 → 編號的雜湊索引。
        index: Table<u32>,
    },
}
