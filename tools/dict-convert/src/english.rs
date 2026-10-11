//! 英文詞表：每行 `詞\t編碼[\t…]`（表頭行、YAML 頭這類沒有製表符或編碼不是字母的行自然跳過），`#` 為註釋。只收純字母的詞。
//!
//! 多個輸入檔案時：同一檔案內同一編碼**先到先得**（資料包按常用度排序，第一個就是最常用的寫法），
//! 後面的檔案可以**覆蓋**前面已經收過的編碼——展示寫法補充表（`07_display_forms.tsv`：`Windows` / `GitHub` / `VSCode`）
//! 就是靠這條覆蓋全小寫的普通詞形，而不動別的詞的順序。

use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::ConvertError;

/// `frequency` 是可選的 `編碼\t詞頻` 表，有就寫進第三列。
pub fn convert(
    inputs: &[PathBuf],
    frequency: Option<&Path>,
    output: &Path,
) -> Result<(), ConvertError> {
    let frequencies: HashMap<String, u32> = match frequency {
        Some(path) => std::fs::read_to_string(path)?
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| {
                let (code, count) = l.split_once('\t')?;
                Some((code.to_owned(), count.trim().parse().ok()?))
            })
            .collect(),
        None => HashMap::new(),
    };
    // 編碼 → 在 `rows` 裡的下標；`rows` 按首次出現的順序，輸出順序與輸入一致
    let mut index_by_code: HashMap<String, usize> = HashMap::new();
    let mut rows: Vec<(String, String)> = Vec::new();
    for path in inputs {
        let before = rows.len();
        let mut seen_in_file: HashSet<String> = HashSet::new();
        for raw in std::fs::read_to_string(path)?.lines() {
            let line = raw.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let word = fields.next().unwrap_or_default().trim();
            let Some(code) = fields.next().map(str::trim) else {
                continue;
            };
            // 資料包的表頭：`word\tkey\tfrequency_level\t…`
            if word == "word" && code == "key" {
                continue;
            }
            let code = code.to_ascii_lowercase();
            if word.is_empty() || !word.bytes().all(|b| b.is_ascii_alphabetic()) {
                continue;
            }
            // 同一檔案內先到先得（資料包按常用度排序）
            if !seen_in_file.insert(code.clone()) {
                continue;
            }
            match index_by_code.get(&code) {
                // 後面的檔案覆蓋前面的：展示寫法補充表用這條把 `windows` 頂成 `Windows`
                Some(&index) => rows[index].0 = word.to_owned(),
                None => {
                    index_by_code.insert(code.clone(), rows.len());
                    rows.push((word.to_owned(), code));
                }
            }
        }
        tracing::info!(path = %path.display(), entries = rows.len() - before, "已读取");
    }
    let mut file = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        file,
        "# 由 qingjian-dict-convert english 生成。词\\t编码\\t词频（wordfreq 的 Zipf 频率 ×1000）"
    )?;
    for (word, code) in &rows {
        let count = frequencies.get(code).copied().unwrap_or(0);
        writeln!(file, "{word}\t{code}\t{count}")?;
    }
    file.flush()?;
    tracing::info!(path = %output.display(), entries = rows.len(), "写出完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(name: &str, body: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("qingjian-english-{}-{name}", std::process::id()));
        std::fs::write(&path, body).unwrap();
        path
    }

    fn converted(tag: &str, inputs: &[PathBuf]) -> Vec<(String, String)> {
        let out = std::env::temp_dir().join(format!(
            "qingjian-english-{}-{tag}-out.tsv",
            std::process::id()
        ));
        convert(inputs, None, &out).unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        let rows: Vec<(String, String)> = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| {
                let mut fields = l.split('\t');
                Some((fields.next()?.to_owned(), fields.next()?.to_owned()))
            })
            .collect();
        std::fs::remove_file(out).unwrap();
        rows
    }

    #[test]
    fn skips_the_header_and_keeps_first_seen_casing_within_a_file() {
        let a = write(
            "a.tsv",
            "word\tkey\tfrequency_level\nwindows\twindows\t35\nWindows\twindows\t50\nact\tact\t5300\n",
        );
        let rows = converted("header", std::slice::from_ref(&a));
        std::fs::remove_file(a).unwrap();
        // 表頭不進來；同一檔案內先到先得，`windows` 保持小寫、`act` 的小寫寫法也沒被 `ACT` 頂掉
        assert_eq!(
            rows,
            [
                ("windows".to_owned(), "windows".to_owned()),
                ("act".to_owned(), "act".to_owned())
            ]
        );
    }

    #[test]
    fn later_files_override_and_reuse_the_existing_position() {
        let a = write("a2.tsv", "windows\twindows\nact\tact\n");
        let b = write("b2.tsv", "word\tkey\nWindows\twindows\nVSCode\tvscode\n");
        let rows = converted("override", &[a.clone(), b.clone()]);
        std::fs::remove_file(a).unwrap();
        std::fs::remove_file(b).unwrap();
        // 覆蓋保持原來的位置與順序，新編碼追加在後
        assert_eq!(
            rows,
            [
                ("Windows".to_owned(), "windows".to_owned()),
                ("act".to_owned(), "act".to_owned()),
                ("VSCode".to_owned(), "vscode".to_owned()),
            ]
        );
    }
}
