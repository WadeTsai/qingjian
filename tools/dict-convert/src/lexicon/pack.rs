//! 讀「輸入法字詞庫_分類整理版」目錄：`01_characters/standard_8105.tsv`、`02_common/modern_chinese_common_words.tsv`、`03_domains/*.tsv`。
//! 三種 TSV 首行都是欄位名：`詞條\t拼音\t排序號\t文件頻次\t字表級別\t來源`。

use std::path::Path;

use super::tone::numeric_syllables;
use crate::error::ConvertError;

/// 規範字表的一行。
#[derive(Debug, Clone)]
pub struct CharRow {
    /// 字。
    pub ch: char,

    /// 字表級別 1–3。
    pub level: u8,
}

/// 通用詞表的一行。
#[derive(Debug, Clone)]
pub struct CommonRow {
    /// 詞。
    pub text: String,

    /// 音節（已規範化）。
    pub syllables: Vec<String>,

    /// 原詞號，越小越常用。
    pub rank: u32,
}

/// 領域詞表的一行。
#[derive(Debug, Clone)]
pub struct DomainRow {
    /// 詞。
    pub text: String,

    /// 文件頻次（THUOCL DF），缺失為 0。
    pub df: u64,

    /// 來自 `03_domains/` 的哪個檔案（檔名主幹，如 `law`）；語料挖出的額外詞為 `None`，一律留在基礎詞庫。
    pub domain: Option<String>,

    /// 給定的讀音（短語層由成分詞拼出）；`None` 按字推。
    pub syllables: Option<Vec<String>>,
}

/// 整個資料包。
#[derive(Debug, Default)]
pub struct Pack {
    pub chars: Vec<CharRow>,

    pub common: Vec<CommonRow>,

    pub domain: Vec<DomainRow>,
}

impl Pack {
    pub fn load(dir: &Path) -> Result<Self, ConvertError> {
        let mut pack = Self::default();
        for line in rows(&dir.join("01_characters/standard_8105.tsv"))? {
            let fields: Vec<&str> = line.split('\t').collect();
            let Some(ch) = fields.first().and_then(|f| clean(f).chars().next()) else {
                continue;
            };
            let level = fields
                .get(4)
                .and_then(|f| f.trim().parse().ok())
                .unwrap_or(3);
            pack.chars.push(CharRow { ch, level });
        }
        for line in rows(&dir.join("02_common/modern_chinese_common_words.tsv"))? {
            let fields: Vec<&str> = line.split('\t').collect();
            let (Some(text), Some(pinyin)) = (fields.first(), fields.get(1)) else {
                continue;
            };
            let text = clean(text);
            let syllables = numeric_syllables(pinyin);
            // 拼音與字數對不上的（阿Ｑ、帶逗號的成語）不要
            if text.is_empty()
                || syllables.len() != text.chars().count()
                || !text.chars().all(is_han)
            {
                continue;
            }
            let rank = fields
                .get(2)
                .and_then(|f| f.trim().parse().ok())
                .unwrap_or(u32::MAX);
            pack.common.push(CommonRow {
                text,
                syllables,
                rank,
            });
        }
        let domain_dir = dir.join("03_domains");
        let mut files: Vec<_> = std::fs::read_dir(&domain_dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| is_source_tsv(p))
            .collect();
        files.sort();
        for file in files {
            let before = pack.domain.len();
            let stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            for line in rows(&file)? {
                let fields: Vec<&str> = line.split('\t').collect();
                let Some(text) = fields.first().map(|f| clean(f)) else {
                    continue;
                };
                if text.is_empty() || !text.chars().all(is_han) {
                    continue;
                }
                let df = fields
                    .get(3)
                    .and_then(|f| f.trim().parse().ok())
                    .unwrap_or(0);
                pack.domain.push(DomainRow {
                    text,
                    df,
                    domain: Some(stem.clone()),
                    syllables: None,
                });
            }
            tracing::info!(file = %file.display(), rows = pack.domain.len() - before, "领域词已读取");
        }
        Ok(pack)
    }
}

/// 是不是一份可用的詞庫源 TSV：副檔名對、是檔案，且不是點開頭（隱藏 / 系統後設資料）或波浪號開頭（Office 等編輯器臨時檔案）。
pub(crate) fn is_source_tsv(path: &Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|e| e == "tsv")
        && !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.') || n.starts_with('~'))
}

/// 讀一個帶表頭的 TSV，跳過首行、空行。
fn rows(path: &Path) -> Result<Vec<String>, ConvertError> {
    let source = std::fs::read_to_string(path)?;
    Ok(source
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect())
}

/// 去掉 BOM 與首尾空白。
fn clean(field: &str) -> String {
    field
        .trim()
        .trim_start_matches('\u{feff}')
        .trim()
        .to_owned()
}

/// 漢字（基本區 + 擴充套件 A）。
fn is_han(c: char) -> bool {
    matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 建一個最小詞庫目錄：01/02 表只有表頭（讀出來是空），03_domains 由測試自己放檔案。
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("qingjian-dict-convert-lexicon-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["01_characters", "02_common", "03_domains"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        std::fs::write(
            dir.join("01_characters/standard_8105.tsv"),
            "字\t拼音\t排序号\t文档频次\t字表级别\t来源\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("02_common/modern_chinese_common_words.tsv"),
            "词条\t拼音\t排序号\t文档频次\t字表级别\t来源\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn domain_scan_skips_hidden_temp_and_non_file_entries() {
        let dir = scratch("domain-scan");
        std::fs::write(
            dir.join("03_domains/law.tsv"),
            "词条\t拼音\t排序号\t文档频次\t字表级别\t来源\n法院\tfa yuan\t1\t100\t\t\n",
        )
        .unwrap();
        // 點開頭（隱藏 / 系統後設資料）與波浪號開頭（Office 等臨時檔案）的 TSV 不應被當成領域詞庫
        std::fs::write(
            dir.join("03_domains/.hidden.tsv"),
            "词条\t拼音\t排序号\t文档频次\t字表级别\t来源\n坏词\thuai ci\t1\t100\t\t\n",
        )
        .unwrap();
        std::fs::write(dir.join("03_domains/~$law.tsv"), "不是表\n").unwrap();
        // 同名目錄也不該被當成檔案讀
        std::fs::create_dir(dir.join("food.tsv")).unwrap();

        let pack = Pack::load(&dir).unwrap();
        let domains: Vec<&str> = pack
            .domain
            .iter()
            .filter_map(|r| r.domain.as_deref())
            .collect();
        assert_eq!(domains, ["law"]);
        let words: Vec<&str> = pack.domain.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(words, ["法院"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
