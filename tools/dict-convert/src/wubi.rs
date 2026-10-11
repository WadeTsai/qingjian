//! 形碼碼錶：Rime `.dict.yaml`（極點 86 五筆等）→ `詞\t編碼\t詞頻`。
//!
//! 碼錶自帶的第二列（Rime 裡是權重）**不用**：那是碼錶順序，不是語料詞頻，直接拿來排序會讓同一個詞
//! 在形碼下和在拼音下排得不一樣。詞頻從青簡詞庫（`詞\t拼音\t詞頻`）按**詞面**交叉回填：
//! 詞庫裡有的用語料詞頻（與拼音方案同一把尺子，釋義兜底與生詞識別也對得上），
//! 沒有的給 [`UNKNOWN_FREQUENCY`]——語料裡一次都沒出現過，本來就該排在同編碼的已知詞後面。
//!
//! 片語編碼（二字詞 2+2、三字詞 1+1+1）由上游碼錶給全，這裡不做取碼推導，見 `docs/plan/wubi.md`。

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use qingjian_dictionary::Dictionary;
use qingjian_dictionary::import::{looks_like_rime, to_tsv};

use crate::error::ConvertError;

/// 詞庫裡查不到的詞給的詞頻。
pub const UNKNOWN_FREQUENCY: u32 = 1;

/// 轉換結果，打日誌用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Converted {
    /// 寫出的詞條數。
    pub entries: usize,

    /// 其中詞頻來自語料的條數。
    pub with_frequency: usize,

    /// 詞庫裡查不到、拿了 [`UNKNOWN_FREQUENCY`] 的條數。佔比高說明碼錶與詞庫的詞面寫法對不上，要查。
    pub unknown: usize,
}

/// 讀 Rime 碼錶寫出青簡形碼 TSV，詞頻從 `frequencies`（青簡詞庫 TSV，可給多本）交叉回填。
pub fn convert(
    input: &Path,
    frequencies: &[PathBuf],
    output: &Path,
) -> Result<Converted, ConvertError> {
    let source = std::fs::read_to_string(input)?;
    if !looks_like_rime(&source) {
        return Err(ConvertError::Format {
            path: input.to_owned(),
            line: 0,
            reason: "expected a Rime .dict.yaml (a `---` header or a `name:` line)".to_owned(),
        });
    }
    // 基礎詞庫加隨包的領域詞庫：詞的語料詞頻與它收在哪一本里無關，同一個詞取最大的那份
    let dictionaries: Vec<Dictionary> = frequencies
        .iter()
        .map(Dictionary::from_path)
        .collect::<Result<_, _>>()?;
    let mut table: HashMap<&str, u32> = HashMap::new();
    for dictionary in &dictionaries {
        for hit in dictionary.entries() {
            let slot = table.entry(hit.text).or_insert(hit.frequency);
            *slot = (*slot).max(hit.frequency);
        }
    }

    let tsv = to_tsv(&source).tsv;
    let mut entries: Vec<(String, String, u32)> = Vec::new();
    for (index, raw) in tsv.lines().enumerate() {
        // 只去尾部空白：詞欄位可能是空的（源表裡 全形空格 那條以製表符開頭），
        // 整行 trim 會把開頭的製表符吃掉、後面的列整體左移，編碼被當成詞
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(word), Some(code)) = (fields.next(), fields.next()) else {
            return Err(ConvertError::Format {
                path: input.to_owned(),
                line: index + 1,
                reason: "expected `word<TAB>code`".to_owned(),
            });
        };
        let (word, code) = (word.trim(), code.trim().to_ascii_lowercase());
        if word.is_empty() || code.is_empty() {
            continue;
        }
        let weight = table.get(word).copied().unwrap_or(UNKNOWN_FREQUENCY);
        entries.push((word.to_owned(), code, weight));
    }
    // 按 (編碼, 詞頻降序, 詞) 排：與 `CodeTable` 在記憶體裡的順序一致，檔案本身也一眼看得出排序；
    // 同一個 (詞, 編碼) 出現兩次（合併多份碼錶）留第一次
    entries.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| a.0.cmp(&b.0))
    });
    entries.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    let unknown = entries
        .iter()
        .filter(|(_, _, frequency)| *frequency == UNKNOWN_FREQUENCY)
        .count();
    let mut writer = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        writer,
        "# 形码码表：词\t编码\t词频。词频由青简词库按词面回填（不是码表自带的权重），出处见 assets/wubi/README.md"
    )?;
    for (word, code, frequency) in &entries {
        writeln!(writer, "{word}\t{code}\t{frequency}")?;
    }
    writer.flush()?;
    Ok(Converted {
        entries: entries.len(),
        with_frequency: entries.len() - unknown,
        unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = r#"---
name: wubi86
...
的	r	100
一	ggll	90
开	ga	80
开发	gant	70
不给	gqt	60
"#;

    /// 詞庫裡有 的 / 開 / 開發 / 一，碼錶裡的 不給 不在其中，用來驗證按詞面回填與兜底。
    const DICT: &str = "的\tde\t5000\n开\tkai\t800\n开发\tkai fa\t900\n一\tyi\t4000\n";

    /// 每個用例一個自己的目錄：`cargo test` 並行跑，共用目錄會互相覆蓋。
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("qingjian-wubi-{name}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn convert_to_strings(name: &str, table: &str, dict: &str) -> (Vec<String>, Converted) {
        let dir = scratch(name);
        std::fs::create_dir_all(&dir).unwrap();
        let table_path = dir.join("table.dict.yaml");
        let dict_path = dir.join("dict.tsv");
        let out_path = dir.join("wubi86.tsv");
        std::fs::write(&table_path, table).unwrap();
        std::fs::write(&dict_path, dict).unwrap();
        let converted = convert(&table_path, std::slice::from_ref(&dict_path), &out_path).unwrap();
        let lines: Vec<String> = std::fs::read_to_string(&out_path)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(str::to_owned)
            .collect();
        (lines, converted)
    }

    #[test]
    fn fills_frequency_by_word_and_sorts_by_code() {
        let (lines, converted) = convert_to_strings("fills", TABLE, DICT);
        assert_eq!(
            lines,
            [
                "开\tga\t800",
                "开发\tgant\t900",
                "一\tggll\t4000",
                "不给\tgqt\t1",
                "的\tr\t5000",
            ]
        );
        assert_eq!(
            converted,
            Converted {
                entries: 5,
                with_frequency: 4,
                unknown: 1,
            }
        );
    }

    #[test]
    fn words_missing_from_the_dictionary_get_the_floor_frequency() {
        let (lines, converted) =
            convert_to_strings("floor", TABLE, "差不多的词\tcha bu duo de ci\t9\n");
        assert!(lines.iter().all(|l| l.ends_with("\t1")));
        assert_eq!(converted.unknown, 5);
        assert_eq!(converted.with_frequency, 0);
    }

    #[test]
    fn keeps_the_highest_frequency_when_a_word_appears_twice() {
        let (lines, _) =
            convert_to_strings("dupes", "---\nname: t\n...\n开发\tgant\n开发\tgant\n", DICT);
        assert_eq!(lines, ["开发\tgant\t900"]);
    }

    #[test]
    fn skips_rows_whose_word_field_is_empty() {
        // 極點碼錶裡 全形空格 那條就是這個形狀：詞欄位是空的全形空格，行以製表符開頭。
        // 整行 trim 會把製表符吃掉，變成「編碼 cokg 的詞頻 10」這種垃圾條目。
        let (lines, converted) = convert_to_strings(
            "empty-word",
            "---\nname: t\n...\n\tcokg\t10\t全角空格\n开\tga\t80\n",
            DICT,
        );
        assert_eq!(lines, ["开\tga\t800"]);
        assert_eq!(converted.entries, 1);
    }

    #[test]
    fn rejects_files_that_are_not_rime_dictionaries() {
        let dir = scratch("not-rime");
        let table = dir.join("plain.tsv");
        let dict_path = dir.join("dict.tsv");
        std::fs::write(&table, "开\tga\t800\n").unwrap();
        std::fs::write(&dict_path, DICT).unwrap();
        assert!(
            convert(
                &table,
                std::slice::from_ref(&dict_path),
                &dir.join("out.tsv")
            )
            .is_err()
        );
    }

    #[test]
    fn takes_the_highest_frequency_across_dictionaries() {
        // 基礎詞庫沒有 葡萄牙，領域詞庫有：兩本都傳進去就該命中，取大的那份
        let dir = scratch("multi");
        let table_path = dir.join("table.dict.yaml");
        let base_path = dir.join("dict.tsv");
        let domain_path = dir.join("places.tsv");
        let out_path = dir.join("wubi86.tsv");
        std::fs::write(&table_path, "---\nname: t\n...\n葡萄牙\taaah\t20\n").unwrap();
        std::fs::write(&base_path, "开\tkai\t800\n").unwrap();
        std::fs::write(&domain_path, "葡萄牙\tpu tao ya\t300\n").unwrap();
        let converted = convert(&table_path, &[base_path, domain_path], &out_path).unwrap();
        assert_eq!(converted.with_frequency, 1);
        assert_eq!(
            std::fs::read_to_string(&out_path)
                .unwrap()
                .lines()
                .filter(|l| !l.starts_with('#'))
                .collect::<Vec<_>>(),
            ["葡萄牙\taaah\t300"]
        );
    }
}
