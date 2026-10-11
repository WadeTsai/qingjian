use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use jiff::civil::Date;
use qingjian_core::storage::{read_text_lossy, write_atomic_str};
use qingjian_core::{FRESH_UNTIL, Language, LevelCount, VocabularySummary, VocabularyTracker};
use qingjian_translate::LevelTable;

use crate::error::LearningError;

/// 「最近」算幾天（含今天）。
const RECENT_DAYS: i64 = 7;

/// 詞彙記錄的落盤：一個譯詞一行 `語言\t譯詞\t看到輪次\t上屏次數\t用過次數\t首見\t末見`，只寫在本機資料目錄裡。
///
/// 檔案壞了按行跳過，讀不了就只在記憶體裡記（記一次警告），輸入不受影響。
#[derive(Debug, Default)]
pub struct VocabularyBook {
    /// (語言程式碼, 譯詞) → 記錄，按鍵排好。
    entries: BTreeMap<(String, String), Entry>,

    /// 自上次儲存後有沒有新記錄。
    dirty: bool,

    /// [`VocabularyTracker::flush`] 時寫回的路徑；`None` 只在記憶體裡記。
    path: Option<PathBuf>,

    /// 各語言的等級表，彙總時按級數詞；沒有的語言不分級。
    levels: Vec<(Language, LevelTable)>,
}

/// 一條譯詞的記錄。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    /// 上屏時在候選窗口裡的輪次。
    seen: u32,

    /// 上屏過帶它的候選幾次。
    committed: u32,

    /// 直接打出過幾次。
    used: u32,

    /// 第一次記錄的日期。
    first: Date,

    /// 最近一次記錄的日期。
    last: Date,
}

impl Entry {
    fn new(date: Date) -> Self {
        Self {
            seen: 0,
            committed: 0,
            used: 0,
            first: date,
            last: date,
        }
    }
}

impl VocabularyBook {
    /// 從檔案載入（不存在就從零開始，flush 時建）；讀不了就退回只在記憶體裡記。
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        match read_text_lossy(&path) {
            Ok(text) => Self {
                entries: text.as_deref().map(parse).unwrap_or_default(),
                dirty: false,
                path: Some(path),
                levels: Vec::new(),
            },
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "词汇记录读不了，本次只在内存里记");
                Self::default()
            }
        }
    }

    /// 給某種語言配等級表（`levels-<語言>.tsv`），彙總裡就有按級的數字。
    pub fn with_levels(mut self, language: Language, table: LevelTable) -> Self {
        self.levels.retain(|(l, _)| *l != language);
        self.levels.push((language, table));
        self
    }

    /// 記錄的譯詞數（所有語言）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn entry(&mut self, language: Language, word: &str, date: Date) -> &mut Entry {
        self.dirty = true;
        let entry = self
            .entries
            .entry((language.code().to_owned(), word.to_owned()))
            .or_insert_with(|| Entry::new(date));
        entry.last = date;
        entry
    }

    /// 記一次看到（[`VocabularyTracker::record_exposure`] 記在今天）。
    pub fn record_exposure_on(&mut self, language: Language, word: &str, date: Date) {
        self.entry(language, word, date).seen += 1;
    }

    /// 記一次上屏（[`VocabularyTracker::record_commit`] 記在今天）。
    pub fn record_commit_on(&mut self, language: Language, word: &str, used: bool, date: Date) {
        let entry = self.entry(language, word, date);
        entry.committed += 1;
        entry.used += u32::from(used);
    }

    /// 以 `today` 為準的彙總（[`VocabularyTracker::summary`] 以本機今天為準）。
    pub fn summary_on(&self, language: Language, today: Date) -> VocabularySummary {
        let recent_from = today.saturating_sub(jiff::Span::new().days(RECENT_DAYS - 1));
        let table = self
            .levels
            .iter()
            .find(|(l, _)| *l == language)
            .map(|(_, table)| table);
        let mut summary = VocabularySummary::default();
        if let Some(table) = table {
            summary.levels = table
                .levels()
                .iter()
                .enumerate()
                .map(|(rank, name)| LevelCount {
                    name: name.clone(),
                    total: table.size(rank),
                    ..LevelCount::default()
                })
                .collect();
        }
        for ((code, word), entry) in &self.entries {
            if code != language.code() {
                continue;
            }
            summary.seen += 1;
            summary.familiar += u64::from(entry.seen >= FRESH_UNTIL);
            summary.committed += u64::from(entry.committed > 0);
            summary.used += u64::from(entry.used > 0);
            summary.new_this_week += u64::from(entry.first >= recent_from && entry.first <= today);
            if let Some(rank) = table.and_then(|table| table.rank(word))
                && let Some(level) = summary.levels.get_mut(rank)
            {
                level.seen += 1;
                level.familiar += u64::from(entry.seen >= FRESH_UNTIL);
                level.committed += u64::from(entry.committed > 0);
            }
        }
        summary
    }

    /// 寫回檔案（沒有新記錄就什麼都不做）。
    pub fn save(&mut self) -> Result<(), LearningError> {
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

    fn save_to(&self, path: &Path) -> Result<(), LearningError> {
        let mut text = String::from("# 语言\t译词\t看到轮次\t上屏次数\t用过次数\t首见\t末见\n");
        for ((code, word), entry) in &self.entries {
            let _ = writeln!(
                text,
                "{code}\t{word}\t{}\t{}\t{}\t{}\t{}",
                entry.seen, entry.committed, entry.used, entry.first, entry.last
            );
        }
        write_atomic_str(path, &text)?;
        Ok(())
    }
}

/// 按行解析；格式不對的行跳過（記警告），重複的鍵取後一條。
fn parse(text: &str) -> BTreeMap<(String, String), Entry> {
    let mut entries = BTreeMap::new();
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_line(line) {
            Some((key, entry)) => {
                entries.insert(key, entry);
            }
            None => tracing::warn!(line = number + 1, "词汇记录有坏行，跳过"),
        }
    }
    entries
}

fn parse_line(line: &str) -> Option<((String, String), Entry)> {
    let mut fields = line.split('\t');
    let code = fields.next()?.to_owned();
    let word = fields.next()?.to_owned();
    if code.is_empty() || word.is_empty() {
        return None;
    }
    let mut number = || fields.next()?.parse::<u32>().ok();
    let seen = number()?;
    let committed = number()?;
    let used = number()?;
    let first: Date = fields.next()?.parse().ok()?;
    let last: Date = fields.next()?.parse().ok()?;
    Some((
        (code, word),
        Entry {
            seen,
            committed,
            used,
            first,
            last,
        },
    ))
}

fn today() -> Date {
    jiff::Zoned::now().date()
}

impl VocabularyTracker for VocabularyBook {
    fn exposures(&self, language: Language, word: &str) -> u32 {
        self.entries
            .get(&(language.code().to_owned(), word.to_owned()))
            .map_or(0, |entry| entry.seen)
    }

    fn record_exposure(&mut self, language: Language, word: &str) {
        self.record_exposure_on(language, word, today());
    }

    fn record_commit(&mut self, language: Language, word: &str, used: bool) {
        self.record_commit_on(language, word, used, today());
    }

    fn flush(&mut self) {
        if let Err(error) = self.save() {
            tracing::warn!(%error, "词汇记录保存失败");
        }
    }

    fn summary(&self, language: Language) -> VocabularySummary {
        self.summary_on(language, today())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    #[test]
    fn counts_exposures_and_summarizes_per_language() {
        let mut book = VocabularyBook::default();
        let en = Language::English;
        for _ in 0..FRESH_UNTIL {
            book.record_exposure_on(en, "develop", date("2026-08-01"));
        }
        book.record_exposure_on(en, "hello", date("2026-09-05"));
        book.record_commit_on(en, "hello", false, date("2026-09-05"));
        book.record_commit_on(en, "world", true, date("2026-09-06"));
        book.record_exposure_on(Language::Japanese, "開発する", date("2026-09-06"));
        assert_eq!(book.exposures(en, "develop"), FRESH_UNTIL);
        assert_eq!(book.exposures(en, "nothing"), 0);
        let summary = book.summary_on(en, date("2026-09-06"));
        assert_eq!(
            summary,
            VocabularySummary {
                seen: 3,
                familiar: 1,
                committed: 2,
                used: 1,
                new_this_week: 2,
                levels: Vec::new(),
            }
        );
        assert_eq!(
            book.summary_on(Language::Japanese, date("2026-09-06")).seen,
            1
        );
    }

    #[test]
    fn splits_the_summary_by_level_when_a_table_is_attached() {
        let table =
            LevelTable::parse("# levels\tA1\tA2\nhello\tA1\nworld\tA1\ndevelop\tA2\n").unwrap();
        let mut book = VocabularyBook::default().with_levels(Language::English, table);
        let en = Language::English;
        for _ in 0..FRESH_UNTIL {
            book.record_exposure_on(en, "develop", date("2026-09-01"));
        }
        book.record_exposure_on(en, "hello", date("2026-09-06"));
        book.record_commit_on(en, "hello", false, date("2026-09-06"));
        book.record_exposure_on(en, "unlisted", date("2026-09-06"));
        let summary = book.summary_on(en, date("2026-09-06"));
        assert_eq!(summary.seen, 3);
        assert_eq!(
            summary.levels,
            vec![
                LevelCount {
                    name: "A1".into(),
                    total: 2,
                    seen: 1,
                    familiar: 0,
                    committed: 1,
                },
                LevelCount {
                    name: "A2".into(),
                    total: 1,
                    seen: 1,
                    familiar: 1,
                    committed: 0,
                },
            ]
        );
    }

    #[test]
    fn round_trips_through_the_file_and_skips_bad_lines() {
        let dir = std::env::temp_dir().join(format!("qingjian-vocab-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user-vocab.tsv");
        std::fs::write(
            &path,
            "# 头\nen\tdevelop\t2\t1\t0\t2026-09-01\t2026-09-05\n坏行\nen\t\t1\t0\t0\t2026-09-01\t2026-09-01\n",
        )
        .unwrap();
        let mut book = VocabularyBook::open(&path);
        assert_eq!(book.len(), 1);
        assert_eq!(book.exposures(Language::English, "develop"), 2);
        book.save().unwrap();
        book.record_exposure_on(Language::English, "develop", date("2026-09-06"));
        book.record_commit_on(Language::English, "world", true, date("2026-09-06"));
        book.save().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            "# 语言\t译词\t看到轮次\t上屏次数\t用过次数\t首见\t末见\n\
             en\tdevelop\t3\t1\t0\t2026-09-01\t2026-09-06\n\
             en\tworld\t0\t1\t1\t2026-09-06\t2026-09-06\n"
        );
        let reloaded = VocabularyBook::open(&path);
        assert_eq!(
            reloaded
                .summary_on(Language::English, date("2026-09-06"))
                .seen,
            2
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
