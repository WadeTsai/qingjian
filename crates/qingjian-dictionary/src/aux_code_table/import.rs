//! 匯入碼錶：把使用者給的 Rime `.dict.yaml`（含 `import_tables` 合表）或現成的 `.qj` 變成碼錶目錄裡的一個 `.qj`。
//!
//! 與詞庫匯入的差別：碼錶不能靜默丟資料。有詞沒有碼的行、碼不是 `a-z` 的行都進統計返回給呼叫方，
//! 一個可用條目都沒有時直接報錯（不是碼錶檔案，或碼列全是垃圾）。

use std::path::{Path, PathBuf};

use qingjian_format::{Container, Metadata};

use super::imported::AuxCodeTableImport;
use super::parsed::{ParsedTable, stem_of};
use super::report::AuxCodeTableImportReport;
use super::table::AuxCodeTable;
use crate::error::DictionaryError;

/// `import_tables` 最多跟幾層：互相引用時繞不出來，到這就停。
const MAX_IMPORT_DEPTH: usize = 8;

/// 把 `source` 匯入到 `dest_dir`，返回寫出的檔案、碼錶名與統計。
pub fn import_aux_code_table(
    source: &Path,
    dest_dir: &Path,
) -> Result<AuxCodeTableImport, DictionaryError> {
    let stem = stem_of(source).ok_or(DictionaryError::Corrupt("source has no usable file name"))?;
    std::fs::create_dir_all(dest_dir)?;
    let target = dest_dir.join(format!("{stem}.qj"));
    let (table, mut metadata, report) = if Container::is_qj(source) {
        let table = AuxCodeTable::open(source)?;
        let metadata = table.metadata().cloned().unwrap_or_default();
        let report = AuxCodeTableImportReport {
            read: table.len(),
            with_code: table.len(),
            ..AuxCodeTableImportReport::default()
        };
        (table, metadata, report)
    } else {
        let (parsed, report) = read_all(source)?;
        if parsed.pairs.is_empty() {
            return Err(DictionaryError::NoCodeEntries);
        }
        let table = AuxCodeTable::from_pairs(parsed.pairs)?;
        let metadata = Metadata {
            name: parsed.name.unwrap_or_default(),
            version: parsed.version.unwrap_or_default(),
            source: source.display().to_string(),
            ..Metadata::default()
        };
        (table, metadata, report)
    };
    if metadata.name.is_empty() {
        metadata.name = stem;
    }
    table.write_qj(&target, &metadata)?;
    Ok(AuxCodeTableImport {
        path: target,
        name: metadata.name,
        report,
    })
}

/// 讀主檔案與它 `import_tables` 指到的其他碼錶，合併成一份解析結果與一份統計。
fn read_all(source: &Path) -> Result<(ParsedTable, AuxCodeTableImportReport), DictionaryError> {
    let mut merged = ParsedTable::default();
    let mut report = AuxCodeTableImportReport::default();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut queue: Vec<(PathBuf, usize)> = vec![(source.to_owned(), 0)];
    let mut main = true;
    while let Some((path, depth)) = queue.pop() {
        if depth > MAX_IMPORT_DEPTH {
            tracing::warn!(file = %path.display(), "import_tables 嵌套太深，跳过");
            continue;
        }
        let identity = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if seen.contains(&identity) {
            continue;
        }
        seen.push(identity);
        let parsed = ParsedTable::parse(&std::fs::read_to_string(&path)?);
        report.read += parsed.read;
        report.with_code += parsed.with_code;
        report.no_code += parsed.no_code;
        report.skipped += parsed.skipped;
        if main {
            merged.name = parsed.name.clone();
            merged.version = parsed.version.clone();
            main = false;
        }
        merged.pairs.extend(parsed.pairs);
        let dir = path.parent().unwrap_or(Path::new(".")).to_owned();
        for name in parsed.import_tables.iter().rev() {
            queue.push((resolve_import(&dir, name), depth + 1));
        }
    }
    Ok((merged, report))
}

/// `import_tables` 裡的一條解析成路徑：原樣存在的先用，否則按 Rime 的慣例補 `.dict.yaml`。
fn resolve_import(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if candidate.exists() {
        return candidate;
    }
    for suffix in [".dict.yaml", ".yaml", ".yml"] {
        let candidate = dir.join(format!("{name}{suffix}"));
        if candidate.exists() {
            return candidate;
        }
    }
    candidate
}
