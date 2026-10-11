//! 筆畫碼錶：CNS11643 全字型檔「筆順資料」+ 大陸序覆蓋表 → `codes/stroke.tsv`（隨包筆畫表的源資料）。
//!
//! 產物每行 `字\t序列`：序列按大陸筆順，1 橫 / 2 豎 / 3 撇 / 5 折 / n 點與捺
//! （CNS 的 4 同時涵蓋點與捺，按「點捺合併為 n」寫 n；提在 CNS 裡多數已記 1，氵的第三筆記 4，由覆蓋表改回 1）。
//! 資料鏈：
//!
//! 1. `--cns-map`：官方 CNS→Unicode 對照表（`CNS2UNICODE_Unicode*.txt`，UTF-8、行內容純 ASCII），建 CNS 字碼 → 字；
//! 2. `--cns-seq`：官方筆順資料（`CNS_strokes_sequence.txt`），`CNS 字碼\t[1-5]{n}`，經上一步對映成 字 → 序列
//!    （同一個字落到兩條不同序列上直接報錯：那說明對照表或筆順表讀錯了）；
//! 3. `--cns-count`：官方筆畫數（`CNS_stroke.txt`，可選），序列長度與它自洽（差不超過 `--max-diff`）的字才留——
//!    CNS 的序列與筆畫數在罕用字上互相打架（t07 實測 |d| > 1 約 0.5%），**以序列為準、以筆畫數過濾**；
//! 4. `--prc-rules`：大陸序覆蓋表（`assets/stroke/prc-rules.tsv`），台灣序按部件重寫成大陸序；
//! 5. `--filter`：字表白名單（預設通用規範字表 8,105 字），只出表裡的字，按表序排列。
//!
//! `--verify` 再對照大陸規範：筆畫數按抽樣比對、首筆按字表全量比對，不符的字必須都在白名單裡，
//! 否則退出碼非 0（對照表由 `mmh-reference` 生成到 `data/mmh/`，找不到哪張就跳過哪張）。
//! 資料來源、許可與驗收記錄見 `assets/stroke/README.md`；隨包前由 `pack codes`（見 `codes` 模組）按取碼規則把它與詞庫算成碼錶。

mod options;
mod rules;
mod verify;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::error::ConvertError;
use crate::stroke::rules::PrcRules;
pub(crate) use crate::stroke::rules::{malformed, single_char};

pub use crate::stroke::options::StrokeOptions;

/// 對照表檔名的字首（給目錄用）。
const CNS_MAP_PREFIX: &str = "CNS2UNICODE";

/// CNS 序列裡出現的筆畫碼。
const STROKE_DIGITS: &str = "12345";

/// 生成筆畫表。
pub fn convert(options: &StrokeOptions, out_dir: &Path) -> Result<(), ConvertError> {
    let started = Instant::now();
    let rules = PrcRules::from_path(&options.prc_rules)?;
    tracing::info!(
        rules = rules.rule_count(),
        overrides = rules.override_count(),
        path = %options.prc_rules.display(),
        "已读大陆序覆盖表"
    );
    let chars = read_cns_map(&options.cns_map)?;
    tracing::info!(entries = chars.len(), "已读 CNS→Unicode 对照表");
    let sequences = read_sequences(&options.cns_seq, &chars)?;
    tracing::info!(entries = sequences.len(), "已读筆順資料");
    let stroke_counts = match options.cns_count.as_deref() {
        Some(path) => {
            let counts = read_stroke_counts(path, &chars)?;
            tracing::info!(entries = counts.len(), "已读筆畫數");
            counts
        }
        None => HashMap::new(),
    };
    let table = read_filter(&options.filter)?;
    tracing::info!(chars = table.len(), path = %options.filter.display(), "已读字表白名单");

    let out = options
        .output
        .clone()
        .unwrap_or_else(|| out_dir.join("codes").join("stroke.tsv"));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = BufWriter::new(File::create(&out)?);
    let mut written = 0usize;
    let mut missing = 0usize;
    let mut inconsistent = 0usize;
    let mut digits = 0usize;
    for ch in &table {
        let original = sequences.get(ch);
        if let (Some(sequence), Some(count)) = (original, stroke_counts.get(ch))
            // 整字覆蓋是人工裁定，優先於 CNS 序列與筆畫數的自洽性（誕 這類被自洽過濾丟掉的字靠它補錄）
            && rules.override_sequence(*ch).is_none()
            && sequence.len().abs_diff(*count as usize) > options.max_diff
        {
            inconsistent += 1;
            continue;
        }
        let sequence = match original {
            Some(sequence) => rules.apply(*ch, sequence),
            None => match rules.override_sequence(*ch) {
                Some(sequence) => sequence.to_owned(),
                None => {
                    missing += 1;
                    continue;
                }
            },
        };
        let sequence = normalize(&sequence);
        writeln!(writer, "{ch}\t{sequence}")?;
        written += 1;
        digits += sequence.len();
    }
    writer.flush()?;
    let average = digits as f64 / written.max(1) as f64;

    let size = std::fs::metadata(&out).map(|meta| meta.len()).unwrap_or(0);
    tracing::info!(
        out = %out.display(),
        entries = written,
        dropped_no_sequence = missing,
        dropped_inconsistent = inconsistent,
        average_strokes = average,
        size_kb = size / 1_000,
        elapsed_ms = started.elapsed().as_millis(),
        "已生成笔画表"
    );
    if options.verify {
        verify::run(options, &out)?;
    }
    Ok(())
}

/// 點與捺合併成 `n`：產物只用 1 2 3 5 n 五個符號，與大陸廠商的 h/s/p/n/z 鍵位對得上。
fn normalize(sequence: &str) -> String {
    sequence
        .chars()
        .map(|c| if c == '4' { 'n' } else { c })
        .collect()
}

/// 讀 CNS→Unicode 對照表：可以給檔案，也可以給目錄（目錄取其中的對照表）。
fn read_cns_map(paths: &[PathBuf]) -> Result<HashMap<String, char>, ConvertError> {
    let mut map = HashMap::new();
    for path in expand_map_paths(paths)? {
        for (index, line) in lines_of(&path)?.enumerate() {
            let line = line?;
            let number = index + 1;
            if line.is_empty() {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(code), Some(value)) = (fields.next(), fields.next()) else {
                return Err(malformed(
                    &path,
                    number,
                    "expected a CNS code and a code point",
                ));
            };
            let point = u32::from_str_radix(value, 16)
                .map_err(|_| malformed(&path, number, "code point is not hexadecimal"))?;
            let Some(ch) = char::from_u32(point) else {
                return Err(malformed(&path, number, "code point is not a character"));
            };
            map.entry(code.to_owned()).or_insert(ch);
        }
    }
    Ok(map)
}

/// 把 `--cns-map` 給的路徑展開成檔案列表：目錄取其中以 `CNS2UNICODE` 開頭的 `.txt`，按名字排序。
fn expand_map_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, ConvertError> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(path)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|entry| {
                    entry.extension().is_some_and(|ext| ext == "txt")
                        && entry
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy().starts_with(CNS_MAP_PREFIX))
                })
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(path.clone());
        }
    }
    if files.is_empty() {
        return Err(ConvertError::Format {
            path: paths.first().cloned().unwrap_or_default(),
            line: 0,
            reason: "no CNS to Unicode table found".to_owned(),
        });
    }
    Ok(files)
}

/// 讀筆順資料，經對照表對映成 字 → 序列。
fn read_sequences(
    path: &Path,
    chars: &HashMap<String, char>,
) -> Result<HashMap<char, String>, ConvertError> {
    let mut sequences: HashMap<char, String> = HashMap::new();
    for (index, line) in lines_of(path)?.enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(code), Some(sequence)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a CNS code and a stroke sequence",
            ));
        };
        if sequence.is_empty() || !sequence.chars().all(|c| STROKE_DIGITS.contains(c)) {
            return Err(malformed(
                path,
                number,
                "stroke sequence must be digits 1-5",
            ));
        }
        let Some(ch) = chars.get(code) else {
            continue;
        };
        match sequences.get(ch) {
            Some(known) if known != sequence => {
                return Err(malformed(
                    path,
                    number,
                    "two stroke sequences for one character",
                ));
            }
            Some(_) => {}
            None => {
                sequences.insert(*ch, sequence.to_owned());
            }
        }
    }
    Ok(sequences)
}

/// 讀筆畫數（第二個欄位是 10 進位制的筆畫數）。
fn read_stroke_counts(
    path: &Path,
    chars: &HashMap<String, char>,
) -> Result<HashMap<char, u32>, ConvertError> {
    let mut counts = HashMap::new();
    for (index, line) in lines_of(path)?.enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(code), Some(value)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a CNS code and a stroke count",
            ));
        };
        let count = value
            .parse::<u32>()
            .map_err(|_| malformed(path, number, "stroke count is not a number"))?;
        if let Some(ch) = chars.get(code) {
            counts.entry(*ch).or_insert(count);
        }
    }
    Ok(counts)
}

/// 讀字表白名單：一行一個字（第一列），別的列忽略；表序即輸出序。
fn read_filter(path: &Path) -> Result<Vec<char>, ConvertError> {
    let mut chars = Vec::new();
    let mut seen = HashSet::new();
    for line in lines_of(path)? {
        let line = line?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(ch) = single_char(line.split('\t').next().unwrap_or_default()) else {
            continue;
        };
        if seen.insert(ch) {
            chars.push(ch);
        }
    }
    Ok(chars)
}

/// 按行讀一個檔案。
fn lines_of(path: &Path) -> Result<std::io::Lines<BufReader<File>>, ConvertError> {
    Ok(BufReader::new(File::open(path)?).lines())
}
