use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use qingjian_core::storage::{read_text_lossy, write_atomic_str};
use qingjian_core::{Language, Sense, Translation};

use crate::GlossaryError;
use crate::glossary::parse_sense;

/// 個人釋義表：釋義兜底從雲端寫來的釋義，一種學習語言一個檔案（`user-glossary-<語言>.tsv`），
/// 格式與隨包釋義表相同（`詞\t[詞性. ]譯詞[|讀音]…`），使用者可以手改。
///
/// 載入按行容錯（壞行警告跳過），讀不了就只在記憶體裡記；落盤走原子寫。
#[derive(Debug)]
pub struct PersonalGlossary {
    /// 學習語言。
    language: Language,

    /// 詞 → 譯詞。
    entries: HashMap<String, Translation>,

    /// 落盤路徑；`None` 只在記憶體裡記。
    path: Option<PathBuf>,

    /// 自上次儲存後有沒有新條目。
    dirty: bool,
}

impl PersonalGlossary {
    /// 只在記憶體裡記。
    pub fn in_memory(language: Language) -> Self {
        Self {
            language,
            entries: HashMap::new(),
            path: None,
            dirty: false,
        }
    }

    /// 從檔案載入（不存在就從空開始，flush 時建）；讀不了就退回只在記憶體裡記。
    pub fn open(language: Language, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        match read_text_lossy(&path) {
            Ok(text) => Self {
                language,
                entries: text
                    .as_deref()
                    .map_or_else(HashMap::new, |t| parse(language, t)),
                path: Some(path),
                dirty: false,
            },
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "个人释义表读不了，本次只在内存里记");
                Self::in_memory(language)
            }
        }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn translate(&self, word: &str) -> Option<Translation> {
        self.entries.get(word).cloned()
    }

    /// 記一條（同詞覆蓋）；語言對不上的不收。
    pub fn insert(&mut self, word: &str, translation: Translation) {
        if translation.language != self.language || translation.senses().is_empty() {
            return;
        }
        self.entries.insert(word.to_owned(), translation);
        self.dirty = true;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 寫回檔案（沒有新條目就什麼都不做）。
    pub fn save(&mut self) -> Result<(), GlossaryError> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        if !self.dirty {
            return Ok(());
        }
        self.save_to(&path)?;
        self.dirty = false;
        Ok(())
    }

    fn save_to(&self, path: &Path) -> Result<(), GlossaryError> {
        let mut text = format!(
            "# 青简个人释义表（{}）：释义兜底从云端写来的，格式同随包释义表，可手改。词\\t[词性. ]译词[|读音]\n",
            self.language.code()
        );
        let mut words: Vec<&String> = self.entries.keys().collect();
        words.sort();
        for word in words {
            text.push_str(word);
            for sense in self.entries[word].senses() {
                text.push('\t');
                text.push_str(&format_sense(sense));
            }
            text.push('\n');
        }
        write_atomic_str(path, &text)?;
        Ok(())
    }
}

/// `詞性. 譯詞|讀音`，與 [`parse_sense`] 互逆。
fn format_sense(sense: &Sense) -> String {
    let mut out = String::new();
    if let Some(pos) = sense.part_of_speech {
        let _ = write!(out, "{pos} ");
    }
    out.push_str(&sense.text);
    if let Some(reading) = &sense.reading {
        let _ = write!(out, "|{reading}");
    }
    out
}

/// 按行解析，壞行跳過。
fn parse(language: Language, text: &str) -> HashMap<String, Translation> {
    let mut entries = HashMap::new();
    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t').map(str::trim);
        let word = fields.next().unwrap_or_default();
        let senses: Vec<Sense> = fields
            .filter(|s| !s.is_empty())
            .take(Translation::MAX_SENSES)
            .map(parse_sense)
            .collect();
        if word.is_empty() || senses.is_empty() {
            tracing::warn!(line = number + 1, "个人释义表有坏行，跳过");
            continue;
        }
        entries.insert(word.to_owned(), Translation::new(language, senses));
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::PartOfSpeech;

    fn translation(
        language: Language,
        pos: Option<PartOfSpeech>,
        text: &str,
        reading: Option<&str>,
    ) -> Translation {
        Translation::new(
            language,
            vec![Sense {
                part_of_speech: pos,
                text: text.to_owned(),
                reading: reading.map(str::to_owned),
                fresh: false,
            }],
        )
    }

    #[test]
    fn round_trips_through_the_file_in_glossary_format() {
        let dir = std::env::temp_dir().join(format!("qingjian-personal-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user-glossary-ja.tsv");
        std::fs::write(&path, "# 头\n开放\tadj. 開放的|かいほうてき\n坏行\n").unwrap();
        let mut glossary = PersonalGlossary::open(Language::Japanese, &path);
        assert_eq!(glossary.len(), 1);
        assert_eq!(
            glossary.translate("开放").unwrap().senses()[0]
                .reading
                .as_deref(),
            Some("かいほうてき")
        );
        glossary.insert(
            "开发",
            translation(
                Language::Japanese,
                Some(PartOfSpeech::Verb),
                "開発する",
                Some("かいはつする"),
            ),
        );
        // 語言不對的不收
        glossary.insert("测试", translation(Language::English, None, "test", None));
        assert_eq!(glossary.len(), 2);
        glossary.save().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.ends_with("开发\tv. 開発する|かいはつする\n开放\tadj. 開放的|かいほうてき\n"));
        let reloaded = PersonalGlossary::open(Language::Japanese, &path);
        assert_eq!(
            reloaded.translate("开发").unwrap().senses()[0].text,
            "開発する"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
