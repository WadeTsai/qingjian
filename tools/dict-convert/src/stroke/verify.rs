//! 對照驗收：產物逐字對大陸規範，白名單之外一處不符即失敗。
//!
//! 兩張對照表由 `mmh-reference` 生成、放在 `data/mmh/`（Arphic 派生，不進倉庫，見 `assets/stroke/README.md`）；
//! 找不到哪張就跳過哪張對照並提示：
//!
//! - 筆畫數：按 `--stride` 抽樣，比對 `--reference`；
//! - 首筆：字表全量（不抽樣），比對 `--first-reference` 的幾何類別。幾何類別是近似（豎撇走向近豎、
//!   點的走向與撇 / 豎難分），但兩岸筆順的差異也落在這幾類裡，所以不按類別放行：不符的字都要在
//!   `--first-whitelist` 裡。
//!
//! 白名單裡是已知且接受的差異：規範裁定後保留的 CNS 結構性差異，與對照源分筆不同的假陽性。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::ConvertError;
use crate::stroke::options::StrokeOptions;
use crate::stroke::rules::{malformed, single_char};

/// 首筆對照的結果計數。
pub(super) struct FirstOutcome {
    /// 有對照且可判的字數。
    pub(super) compared: usize,

    /// 產物裡沒有的字數（生成時被丟掉的字）。
    pub(super) absent: usize,

    /// 對照表沒收的字數。
    pub(super) unreferenced: usize,

    /// 對照類別是 ?（幾何上判不了）的字數。
    pub(super) incomparable: usize,

    /// 命中白名單的字。
    pub(super) used: HashSet<char>,

    /// 白名單之外的不符數。
    pub(super) unmatched: usize,
}

/// 比對抽樣表裡的筆畫數：返回（抽樣數、命中的白名單字、白名單之外的不符數）。
pub(super) fn compare(
    sampled: &[char],
    produced: &HashMap<char, usize>,
    expected: &HashMap<char, usize>,
    whitelist: &HashSet<char>,
) -> (usize, HashSet<char>, usize) {
    let mut used = HashSet::new();
    let mut unmatched = 0usize;
    for ch in sampled {
        let Some(want) = expected.get(ch) else {
            tracing::warn!(character = %ch, "对照表没有这个字，跳过");
            continue;
        };
        let got = produced.get(ch).copied();
        if got == Some(*want) {
            continue;
        }
        if whitelist.contains(ch) {
            used.insert(*ch);
            tracing::info!(character = %ch, table = ?got, reference = want, "白名单内的已知差异");
            continue;
        }
        unmatched += 1;
        tracing::error!(character = %ch, table = ?got, reference = want, "白名单之外的不符");
    }
    (sampled.len(), used, unmatched)
}

/// 比對字表全量的首筆：類別不一致又不在白名單裡的記不符。
pub(super) fn compare_first(
    chars: &[char],
    produced: &HashMap<char, char>,
    expected: &HashMap<char, char>,
    whitelist: &HashSet<char>,
) -> FirstOutcome {
    let mut outcome = FirstOutcome {
        compared: 0,
        absent: 0,
        unreferenced: 0,
        incomparable: 0,
        used: HashSet::new(),
        unmatched: 0,
    };
    for ch in chars {
        let Some(ours) = produced.get(ch) else {
            outcome.absent += 1;
            tracing::warn!(character = %ch, "产物里没有这个字，跳过首笔对照");
            continue;
        };
        let Some(reference) = expected.get(ch) else {
            outcome.unreferenced += 1;
            tracing::warn!(character = %ch, "首笔对照表没有这个字，跳过");
            continue;
        };
        if *reference == '?' {
            outcome.incomparable += 1;
            continue;
        }
        if *ours == *reference {
            outcome.compared += 1;
            continue;
        }
        if whitelist.contains(ch) {
            outcome.used.insert(*ch);
            tracing::info!(character = %ch, table = %ours, reference = %reference, "白名单内的已知差异");
            continue;
        }
        outcome.unmatched += 1;
        tracing::error!(character = %ch, table = %ours, reference = %reference, "白名单之外的不符");
    }
    outcome
}

/// 按表序每 `stride` 字取一個。
pub(super) fn sample(chars: &[char], stride: usize) -> Vec<char> {
    chars
        .iter()
        .enumerate()
        .filter(|(index, _)| (index + 1) % stride == 0)
        .map(|(_, ch)| *ch)
        .collect()
}

/// 跑一次對照：筆畫數抽樣、首筆全量，白名單之外有不符合就報錯。
pub fn run(options: &StrokeOptions, table: &Path) -> Result<(), ConvertError> {
    if options.stride == 0 {
        return Err(malformed(
            &options.sample,
            0,
            "stride must be greater than zero",
        ));
    }
    let chars = read_chars(&options.sample)?;
    if options.reference.exists() {
        let produced = read_lengths(table)?;
        let expected = read_reference(&options.reference)?;
        let known = read_whitelist(&options.whitelist)?;
        let sampled = sample(&chars, options.stride);
        let (count, used, unmatched) = compare(&sampled, &produced, &expected, &known);
        tracing::info!(
            table = %table.display(),
            sampled = count,
            reference_entries = expected.len(),
            whitelisted = used.len(),
            unmatched,
            "笔画数对照完成"
        );
        if used.len() < known.len() {
            tracing::warn!(
                unused = known.len() - used.len(),
                path = %options.whitelist.display(),
                "白名单里有字这次没有不符，可以删掉"
            );
        }
        if unmatched > 0 {
            return Err(ConvertError::Verify { unmatched });
        }
    } else {
        tracing::warn!(
            reference = %options.reference.display(),
            "找不到笔画数对照表，跳过对照（mmh-reference 生成，见 assets/stroke/README.md）"
        );
    }
    if options.first_reference.exists() {
        let produced = read_firsts(table)?;
        let expected = read_first_reference(&options.first_reference)?;
        let known = read_whitelist(&options.first_whitelist)?;
        let outcome = compare_first(&chars, &produced, &expected, &known);
        tracing::info!(
            table = %table.display(),
            compared = outcome.compared,
            absent = outcome.absent,
            unreferenced = outcome.unreferenced,
            incomparable = outcome.incomparable,
            whitelisted = outcome.used.len(),
            unmatched = outcome.unmatched,
            "首笔对照完成"
        );
        if outcome.used.len() < known.len() {
            tracing::warn!(
                unused = known.len() - outcome.used.len(),
                path = %options.first_whitelist.display(),
                "白名单里有字这次没有不符，可以删掉"
            );
        }
        if outcome.unmatched > 0 {
            return Err(ConvertError::Verify {
                unmatched: outcome.unmatched,
            });
        }
    } else {
        tracing::warn!(
            reference = %options.first_reference.display(),
            "找不到首笔对照表，跳过首笔对照（mmh-reference 生成，见 assets/stroke/README.md）"
        );
    }
    Ok(())
}

/// 讀產物：字 → 序列長度。
fn read_lengths(path: &Path) -> Result<HashMap<char, usize>, ConvertError> {
    let mut lengths = HashMap::new();
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(ch), Some(sequence)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a character and a stroke sequence",
            ));
        };
        let Some(ch) = single_char(ch) else {
            return Err(malformed(path, number, "expected one character"));
        };
        if sequence.is_empty() {
            return Err(malformed(path, number, "stroke sequence is empty"));
        }
        lengths.insert(ch, sequence.chars().count());
    }
    Ok(lengths)
}

/// 讀產物首筆：字 → 鍵位類別（1→h、2→s、3→p、5→z、n→n）。
fn read_firsts(path: &Path) -> Result<HashMap<char, char>, ConvertError> {
    let mut firsts = HashMap::new();
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(ch), Some(sequence)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a character and a stroke sequence",
            ));
        };
        let Some(ch) = single_char(ch) else {
            return Err(malformed(path, number, "expected one character"));
        };
        let class = match sequence.chars().next() {
            Some('1') => 'h',
            Some('2') => 's',
            Some('3') => 'p',
            Some('5') => 'z',
            Some('n') => 'n',
            _ => {
                return Err(malformed(
                    path,
                    number,
                    "stroke sequence starts with an unknown stroke",
                ));
            }
        };
        firsts.insert(ch, class);
    }
    Ok(firsts)
}

/// 讀筆畫數對照表：字 → 大陸筆畫數。
fn read_reference(path: &Path) -> Result<HashMap<char, usize>, ConvertError> {
    let mut counts = HashMap::new();
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(ch), Some(value)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a character and a stroke count",
            ));
        };
        let Some(ch) = single_char(ch) else {
            return Err(malformed(path, number, "expected one character"));
        };
        let count = value
            .parse::<usize>()
            .map_err(|_| malformed(path, number, "stroke count is not a number"))?;
        counts.insert(ch, count);
    }
    Ok(counts)
}

/// 讀首筆對照表：字 → 幾何類別（h / s / p / n / z / ?）。
fn read_first_reference(path: &Path) -> Result<HashMap<char, char>, ConvertError> {
    let mut firsts = HashMap::new();
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let line = line?;
        let number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(ch), Some(class)) = (fields.next(), fields.next()) else {
            return Err(malformed(
                path,
                number,
                "expected a character and a first-stroke class",
            ));
        };
        let Some(ch) = single_char(ch) else {
            return Err(malformed(path, number, "expected one character"));
        };
        if !matches!(class, "h" | "s" | "p" | "n" | "z" | "?") {
            return Err(malformed(
                path,
                number,
                "first-stroke class must be one of h s p n z ?",
            ));
        }
        firsts.insert(ch, class.chars().next().unwrap_or('?'));
    }
    Ok(firsts)
}

/// 讀抽樣字表：一行一個字（第一列），表序即抽樣序。
fn read_chars(path: &Path) -> Result<Vec<char>, ConvertError> {
    let mut chars = Vec::new();
    let mut seen = HashSet::new();
    for line in BufReader::new(File::open(path)?).lines() {
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

/// 讀白名單裡的字（第一列）。
fn read_whitelist(path: &Path) -> Result<HashSet<char>, ConvertError> {
    Ok(read_chars(path)?.into_iter().collect::<HashSet<char>>())
}
