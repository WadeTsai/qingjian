//! CC-CEDICT：`繁體 簡體 [拼音] /釋義1/釋義2/…/`，`#` 為註釋。
//!
//! 同一個簡體字詞可能有多條（不同繁體或讀音），按檔案順序合併釋義。
//! 跳過純粹的交叉引用與變體說明，最多保留兩條。

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::ConvertError;

/// 一條譯文最多保留幾條釋義。
const MAX_SENSES: usize = 2;

/// 釋義開頭出現這些就不是真正的解釋。
const META_PREFIXES: &[&str] = &[
    "variant of ",
    "old variant of ",
    "see ",
    "also written ",
    "CL:",
    "used in ",
];

pub fn convert(input: &Path, output: &Path) -> Result<(), ConvertError> {
    let source = std::fs::read_to_string(input)?;
    // 保持首次出現的順序，便於結果穩定
    let mut order: Vec<String> = Vec::new();
    let mut senses: HashMap<String, Vec<String>> = HashMap::new();
    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((simplified, glosses)) = parse_line(line) else {
            return Err(ConvertError::Format {
                path: input.to_path_buf(),
                line: index + 1,
                reason: "not a CC-CEDICT entry".into(),
            });
        };
        let bucket = senses.entry(simplified.to_owned()).or_insert_with(|| {
            order.push(simplified.to_owned());
            Vec::new()
        });
        // 一條 CEDICT 釋義常用 `;` 串多個意思，拆開各算一條，候選框裡才放得下
        for gloss in glosses
            .iter()
            .flat_map(|g| g.split(';'))
            .map(clean_gloss)
            .filter(|g| !g.is_empty())
        {
            if bucket.len() >= MAX_SENSES {
                break;
            }
            if is_meta(&gloss) || bucket.contains(&gloss) {
                continue;
            }
            bucket.push(gloss);
        }
    }

    let mut file = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        file,
        "# 由 qingjian-dict-convert 从 CC-CEDICT 生成（CC BY-SA 4.0，https://cc-cedict.org）。词\\t译文\\t译文"
    )?;
    let mut written = 0;
    for text in &order {
        let glosses = &senses[text];
        if glosses.is_empty() {
            continue;
        }
        writeln!(file, "{text}\t{}", glosses.join("\t"))?;
        written += 1;
    }
    file.flush()?;
    tracing::info!(path = %output.display(), entries = written, "写出完成");
    Ok(())
}

/// 返回（簡體, 釋義列表）。
fn parse_line(line: &str) -> Option<(&str, Vec<&str>)> {
    let (head, rest) = line.split_once(" [")?;
    let (_traditional, simplified) = head.split_once(' ')?;
    let (_pinyin, glosses) = rest.split_once("] /")?;
    let glosses = glosses.strip_suffix('/')?;
    Some((
        simplified,
        glosses
            .split('/')
            .map(str::trim)
            .filter(|g| !g.is_empty())
            .collect(),
    ))
}

/// 去掉開頭的用法標註（`(bound form)`、`(literary)` 等）和首尾空白。
fn clean_gloss(gloss: &str) -> String {
    let mut g = gloss.trim();
    while g.starts_with('(') {
        match g.find(") ") {
            Some(end) => g = g[end + 2..].trim_start(),
            None => break,
        }
    }
    g.to_owned()
}

fn is_meta(gloss: &str) -> bool {
    META_PREFIXES.iter().any(|p| gloss.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entry_line() {
        let (simplified, glosses) =
            parse_line("中文 中文 [Zhong1 wen2] /Chinese language/Chinese/").unwrap();
        assert_eq!(simplified, "中文");
        assert_eq!(glosses, ["Chinese language", "Chinese"]);
    }

    #[test]
    fn cleans_leading_usage_notes() {
        assert_eq!(clean_gloss(" (bound form) China"), "China");
        assert_eq!(clean_gloss("to develop"), "to develop");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_line("nonsense").is_none());
    }
}
