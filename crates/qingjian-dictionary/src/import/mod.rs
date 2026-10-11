//! 匯入詞庫：把使用者給的檔案（青簡 TSV、Rime `.dict.yaml`、現成的 `.qj`）變成使用者詞庫目錄裡的一個 `.qj`。
//!
//! 匯入時只做格式轉換，不做拼音校驗：不合法的音節查不到而已，不影響別的詞。
//! 目標檔名取原始檔的主幹（`law.dict.yaml` → `law.qj`），已存在就覆蓋（重新匯入即更新）。

mod imported;
mod rime;

use std::path::{Path, PathBuf};

use qingjian_format::{Container, Metadata};

use crate::dictionary::Dictionary;
use crate::error::DictionaryError;

pub use imported::Imported;
// `dict-convert wubi` 也讀 Rime `.dict.yaml`（形碼碼錶的第二列是編碼，轉換方式一樣），共用這一個解析器
pub use rime::{Parsed, looks_like_rime, to_tsv};

/// 把 `source` 匯入到 `dest_dir`，返回寫出的檔案與後設資料。
pub fn import(source: &Path, dest_dir: &Path) -> Result<Imported, DictionaryError> {
    let stem = source
        .file_name()
        .and_then(|n| n.to_str())
        .map(strip_extensions)
        .filter(|s| !s.is_empty())
        .ok_or(DictionaryError::Corrupt("source has no usable file name"))?;
    std::fs::create_dir_all(dest_dir)?;
    let target = dest_dir.join(format!("{stem}.qj"));
    let (dictionary, metadata) = if Container::is_qj(source) {
        let dictionary = Dictionary::open_qj(source)?;
        let metadata = dictionary.metadata().cloned().unwrap_or_default();
        (dictionary, metadata)
    } else {
        let text = std::fs::read_to_string(source)?;
        let (tsv, name) = if rime::looks_like_rime(&text) {
            let parsed = rime::to_tsv(&text);
            (parsed.tsv, parsed.name)
        } else {
            (text, None)
        };
        let dictionary = Dictionary::parse(&tsv)?;
        let metadata = Metadata {
            name: name.unwrap_or_else(|| stem.clone()),
            source: source.display().to_string(),
            ..Metadata::default()
        };
        (dictionary, metadata)
    };
    if dictionary.is_empty() {
        return Err(DictionaryError::Corrupt(
            "no usable entries; expected word and explicit pinyin columns",
        ));
    }
    dictionary.write_qj(&target, &metadata)?;
    Ok(Imported {
        path: target,
        name: metadata.name,
        entries: dictionary.len(),
    })
}

/// `law.dict.yaml` → `law`，`dict.tsv` → `dict`。
pub(crate) fn strip_extensions(file_name: &str) -> String {
    let mut stem = file_name;
    for suffix in [".dict.yaml", ".yaml", ".yml", ".tsv", ".txt", ".qj"] {
        if let Some(s) = stem.strip_suffix(suffix) {
            stem = s;
            break;
        }
    }
    stem.to_owned()
}

/// 詞庫目錄裡的 `.qj` 檔名。
pub fn target_name(stem: &str) -> PathBuf {
    PathBuf::from(format!("{stem}.qj"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_tsv_and_rime_into_qj() {
        let dir = std::env::temp_dir().join(format!("qingjian-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tsv = dir.join("finance.tsv");
        std::fs::write(&tsv, "账套\tzhang tao\t500\n").unwrap();
        let imported = import(&tsv, &dir.join("dicts")).unwrap();
        assert_eq!(imported.name, "finance");
        assert_eq!(imported.entries, 1);
        let dictionary = Dictionary::from_path(&imported.path).unwrap();
        assert_eq!(dictionary.lookup(&["zhang", "tao"], false)[0].text, "账套");
        assert_eq!(dictionary.metadata().unwrap().name, "finance");

        let rime = dir.join("law.dict.yaml");
        std::fs::write(
            &rime,
            "# Rime dictionary\n---\nname: law\nversion: \"1\"\nsort: by_weight\n...\n合同法\the tong fa\t120\n民法典\tmin fa dian\n",
        )
        .unwrap();
        let imported = import(&rime, &dir.join("dicts")).unwrap();
        assert_eq!(imported.path.file_name().unwrap(), "law.qj");
        assert_eq!(imported.name, "law");
        assert_eq!(imported.entries, 2);
        let dictionary = Dictionary::from_path(&imported.path).unwrap();
        assert_eq!(
            dictionary.lookup(&["min", "fa", "dian"], false)[0].text,
            "民法典"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn strips_known_extensions() {
        assert_eq!(strip_extensions("law.dict.yaml"), "law");
        assert_eq!(strip_extensions("dict.tsv"), "dict");
        assert_eq!(strip_extensions("x.qj"), "x");
        assert_eq!(strip_extensions("noext"), "noext");
    }
}
