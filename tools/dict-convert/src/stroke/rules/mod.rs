//! 大陸序覆蓋表：CNS（台灣標準）筆順按部件重寫成大陸規範。
//!
//! 表在 `assets/stroke/prc-rules.tsv`，三類行（`#` 開頭是註釋，TAB 分隔）：
//!
//! - `rule\t部件\tCNS 模式\t大陸模式\t匹配位置`：部件重寫，按行序逐字應用；
//! - `skip\t部件\t字…`：例外字，這些字裡命中的模式不是那個部件，規則不適用；
//! - `char\t字\t序列`：整字覆蓋（CNS 沒有這個字時人工補錄），優先於所有規則。
//!
//! 規則只做模式替換、不猜字形：改不動的整字形差異（台灣標準與大陸規範筆畫數就不同的字）留給抽樣對照的白名單，
//! 見 `assets/stroke/README.md`。

mod anchor;
mod rule;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::error::ConvertError;
use crate::stroke::rules::anchor::Anchor;
use crate::stroke::rules::rule::Rule;

/// CNS 序列裡出現的筆畫碼：1 橫 / 2 豎 / 3 撇 / 4 點與捺 / 5 折。
const STROKE_DIGITS: &str = "12345";

/// 大陸序覆蓋表。
pub struct PrcRules {
    /// 部件重寫規則，按覆蓋表裡的順序。
    rules: Vec<Rule>,

    /// 整字覆蓋：字 → 序列。
    overrides: HashMap<char, String>,
}

impl PrcRules {
    /// 讀覆蓋表。
    pub fn from_path(path: &Path) -> Result<Self, ConvertError> {
        Self::parse(path, BufReader::new(File::open(path)?))
    }

    /// 從任意讀入流解析覆蓋表（測試用）。
    pub(super) fn parse(path: &Path, reader: impl BufRead) -> Result<Self, ConvertError> {
        let mut rules: Vec<Rule> = Vec::new();
        let mut overrides: HashMap<char, String> = HashMap::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            let number = index + 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["rule", name, from, to, anchor] => {
                    rules.push(Rule {
                        name: (*name).to_owned(),
                        from: (*from).to_owned(),
                        to: (*to).to_owned(),
                        anchor: Anchor::parse(anchor)
                            .ok_or_else(|| malformed(path, number, "unknown anchor"))?,
                        skips: HashSet::new(),
                    });
                }
                ["skip", name, chars] => {
                    let rule = rules
                        .iter_mut()
                        .find(|rule| rule.name == *name)
                        .ok_or_else(|| malformed(path, number, "skip refers to an unknown rule"))?;
                    rule.skips
                        .extend(chars.chars().filter(|c| !c.is_whitespace()));
                }
                ["char", ch, seq] => {
                    let Some(ch) = single_char(ch) else {
                        return Err(malformed(path, number, "expected one character"));
                    };
                    if seq.is_empty() || !seq.chars().all(|c| STROKE_DIGITS.contains(c)) {
                        return Err(malformed(path, number, "expected a stroke sequence of 1-5"));
                    }
                    overrides.insert(ch, (*seq).to_owned());
                }
                _ => return Err(malformed(path, number, "expected rule / skip / char")),
            }
        }
        Ok(Self { rules, overrides })
    }

    /// 整字覆蓋的序列（CNS 沒有這個字時人工補錄的那條）。
    pub fn override_sequence(&self, ch: char) -> Option<&str> {
        self.overrides.get(&ch).map(String::as_str)
    }

    /// 把一個字的 CNS 序列改成大陸序。
    pub fn apply(&self, ch: char, seq: &str) -> String {
        if let Some(overridden) = self.overrides.get(&ch) {
            return overridden.clone();
        }
        let mut out = seq.to_owned();
        for rule in &self.rules {
            if rule.skips.contains(&ch) {
                continue;
            }
            if let Some(rewritten) = rule.anchor.rewrite(&out, &rule.from, &rule.to) {
                out = rewritten;
            }
        }
        out
    }

    /// 規則條數，日誌用。
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// 整字覆蓋條數，日誌用。
    pub fn override_count(&self) -> usize {
        self.overrides.len()
    }
}

/// 第一個欄位就是一個字（覆蓋表、字表白名單與筆畫表都按這個口徑認行）。
pub(crate) fn single_char(field: &str) -> Option<char> {
    let mut chars = field.chars();
    let ch = chars.next()?;
    if chars.next().is_none() {
        Some(ch)
    } else {
        None
    }
}

/// 這類表裡的壞行（覆蓋表、筆畫表共用同一套報錯）。
pub(crate) fn malformed(path: &Path, line: usize, reason: &str) -> ConvertError {
    ConvertError::Format {
        path: PathBuf::from(path),
        line,
        reason: reason.to_owned(),
    }
}
