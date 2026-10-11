mod learner_impl;
mod tables;

use std::collections::BTreeMap;

use foldhash::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use qingjian_core::sentence::{Context, UserNgram};
use qingjian_core::storage::{read_text_lossy, write_atomic};
use qingjian_core::{Candidate, Forgotten, Learner};
use qingjian_dictionary::{Dictionary, WordList};

use crate::error::LearningError;

/// 使用者詞檔名，與詞頻檔案放同一目錄；格式與主詞庫 TSV 相同（`詞\t拼音\t詞頻`）。
const USER_WORDS_FILE: &str = "user-words.tsv";

/// 使用者詞在小詞庫裡的詞頻。排序主要靠 weight，這個值只在同 weight 時起作用。
const USER_WORD_FREQUENCY: u32 = 100;

/// 個人 n-gram 檔名，與詞頻檔案同目錄：`前詞\t後詞\t次數`，句首用 `<s>`。
const USER_NGRAM_FILE: &str = "user-ngram.tsv";

/// 按輸入串記的選擇檔名，與詞頻檔案同目錄：`輸入串\t詞\t次數`。
const USER_CHOICES_FILE: &str = "user-choices.tsv";

/// 個人英文詞表檔名，與詞頻檔案同目錄：`詞\t次數`（詞按第一次敲的寫法存）。
const USER_ENGLISH_FILE: &str = "user-english.tsv";

/// 個人敲錯表文件名，與詞頻檔案同目錄：`敲的\t要的\t次數`（音節級，接受過的糾正）。
const USER_TYPOS_FILE: &str = "user-typos.tsv";

/// 按輸入串記的選擇最多存多少條 (輸入串, 詞)，超過就整體減半（忘掉久遠的偏好）。
const MAX_CHOICE_ENTRIES: usize = 50_000;

/// 「回車原樣上屏過」在選擇表裡的記法：詞那一列寫這個標記（尖括號不可能是候選詞）。
const RAW_MARK: &str = "<raw>";

/// 記憶體中的使用者詞頻表。
#[derive(Debug, Default)]
pub struct FrequencyLearner {
    /// 詞 → 使用者選擇次數。
    counts: HashMap<String, u32>,

    /// 自上次儲存後是否有新記錄。
    dirty: bool,

    /// [`Learner::flush`] 時寫回的路徑；`None` 表示只在記憶體裡學習。
    path: Option<PathBuf>,

    /// 使用者詞：詞 → 全拼（空格分隔）。詞庫裡沒有、使用者卻選過的詞（雲聯想接受的等）。
    words: BTreeMap<String, String>,

    /// 使用者片語成的小詞庫，隨 `words` 重建。
    user_dictionary: Option<Dictionary>,

    /// 使用者詞自上次儲存後是否有變化。
    words_dirty: bool,

    /// 個人 n-gram：上屏詞序列的轉移計數（二元 + 三元）。
    ngram: UserNgram,

    /// 個人 n-gram 自上次儲存後是否有變化。
    ngram_dirty: bool,

    /// 輸入串 → (詞 → 在這個輸入串下被選的次數)。
    choices: HashMap<String, HashMap<String, u32>>,

    /// 按輸入串記的選擇自上次儲存後是否有變化。
    choices_dirty: bool,

    /// 個人英文詞：小寫編碼 → (第一次敲的寫法, 次數)。回車原樣上屏的英文詞、選過的英文候選。
    english: BTreeMap<String, (String, u32)>,

    /// 個人英文片語成的詞表，隨 `english` 重建。
    english_list: Option<WordList>,

    /// 個人英文詞自上次儲存後是否有變化。
    english_dirty: bool,

    /// 個人敲錯表：敲的那段字母 → (要的音節 → 接受次數)。
    typos: HashMap<String, HashMap<String, u32>>,

    /// 個人敲錯表自上次儲存後是否有變化。
    typos_dirty: bool,
}

impl FrequencyLearner {
    /// 從 `詞\t次數` 文本建表（只在記憶體裡學習）。壞行跳過。
    pub fn parse(source: &str) -> Self {
        let mut learner = Self::default();
        learner.load_counts(source);
        learner
    }

    /// 從 `詞\t次數` 讀選擇次數，返回跳過的壞行數。
    fn load_counts(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            match line
                .split_once('\t')
                .and_then(|(text, count)| Some((text, count.trim().parse::<u32>().ok()?)))
            {
                Some((text, count)) => {
                    self.counts.insert(text.to_owned(), count);
                }
                None => skipped += 1,
            }
        }
        skipped
    }

    /// 從檔案載入，之後 [`Learner::flush`] 會寫回同一個檔案。
    /// 檔案不存在時返回空表，而不是報錯：首次執行沒有使用者資料是正常的。
    ///
    /// 各檔案按行容錯：格式不對的行（崩潰寫壞、手改錯了）記一條警告跳過，其餘照讀，下次落盤時就沒了；
    /// 編碼壞掉的位元組按替換字元讀進來交給按行解析處理。只有真正的 io 錯誤（許可權、壞盤）才返回 `Err`，
    /// 這時殼該退回只在記憶體裡學習，別拿空表覆蓋使用者的檔案。
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, LearningError> {
        let path = path.into();
        let mut learner = Self::default();
        if let Some(source) = read_text_lossy(&path)? {
            let skipped = learner.load_counts(&source);
            note_skipped(&path, skipped);
        }
        let words_path = Self::words_path(&path);
        if let Some(source) = read_text_lossy(&words_path)? {
            let skipped = learner.load_words(&source);
            note_skipped(&words_path, skipped);
        }
        let ngram_path = Self::ngram_path(&path);
        if let Some(source) = read_text_lossy(&ngram_path)? {
            let (ngram, skipped) = UserNgram::parse_lenient(&source);
            learner.ngram = ngram;
            note_skipped(&ngram_path, skipped.len());
        }
        let choices_path = Self::choices_path(&path);
        if let Some(source) = read_text_lossy(&choices_path)? {
            let skipped = learner.load_choices(&source);
            note_skipped(&choices_path, skipped);
        }
        let english_path = Self::english_path(&path);
        if let Some(source) = read_text_lossy(&english_path)? {
            let skipped = learner.load_english(&source);
            note_skipped(&english_path, skipped);
        }
        let typos_path = Self::typos_path(&path);
        if let Some(source) = read_text_lossy(&typos_path)? {
            let skipped = learner.load_typos(&source);
            note_skipped(&typos_path, skipped);
        }
        learner.path = Some(path);
        Ok(learner)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn len(&self) -> usize {
        self.counts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    pub fn save_to(&mut self, path: impl AsRef<Path>) -> Result<(), LearningError> {
        let mut rows: Vec<(&String, &u32)> = self.counts.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        write_atomic(path.as_ref(), |file| {
            writeln!(file, "# 青简用户词频：词\\t选择次数")?;
            for (text, count) in rows {
                writeln!(file, "{text}\t{count}")?;
            }
            Ok(())
        })?;
        self.dirty = false;
        Ok(())
    }

    /// 有沒有還沒落盤的學習資料（任何一張表）。
    pub fn has_unsaved(&self) -> bool {
        self.dirty
            || self.words_dirty
            || self.english_dirty
            || self.ngram_dirty
            || self.choices_dirty
            || self.typos_dirty
    }
}

/// 資料行：去掉首尾空白、空行與 `#` 註釋。
fn data_lines(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// 解析 `甲\t乙\t次數` 一行，格式不對返回 `None`。
fn parse_counted_pair(line: &str) -> Option<(&str, &str, u32)> {
    let mut fields = line.split('\t');
    let (Some(first), Some(second), Some(count)) = (fields.next(), fields.next(), fields.next())
    else {
        return None;
    };
    let count = count.trim().parse::<u32>().ok()?;
    Some((first, second, count))
}

/// 載入時跳過了壞行就記一條警告：使用者能從日誌裡知道檔案被寫壞過，下次落盤會把壞行清掉。
fn note_skipped(path: &Path, skipped: usize) {
    if skipped > 0 {
        tracing::warn!(path = %path.display(), skipped, "学习数据里有格式不对的行，已跳过");
    }
}

#[cfg(test)]
mod tests;
