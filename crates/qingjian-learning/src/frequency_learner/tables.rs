//! 各張表的路徑、載入、儲存與重建：使用者詞、按輸入串的選擇、個人英文詞、個人敲錯表、個人 n-gram。

use super::*;

impl FrequencyLearner {
    /// 個人英文詞表檔案與詞頻檔案同目錄。
    pub(super) fn english_path(frequency_path: &Path) -> PathBuf {
        frequency_path.with_file_name(USER_ENGLISH_FILE)
    }

    /// 從 `詞\t次數` 讀個人英文詞，返回跳過的壞行數。
    pub(super) fn load_english(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let Some((word, count)) = line
                .split_once('\t')
                .and_then(|(word, count)| Some((word, count.trim().parse::<u32>().ok()?)))
            else {
                skipped += 1;
                continue;
            };
            if count > 0 {
                self.english
                    .insert(word.to_ascii_lowercase(), (word.to_owned(), count));
            }
        }
        self.rebuild_english();
        skipped
    }

    /// 個人英文詞很少，每次變化整個重建詞表即可。次數當詞頻，補全時常用的在前。
    pub(super) fn rebuild_english(&mut self) {
        if self.english.is_empty() {
            self.english_list = None;
            return;
        }
        let tsv: String = self
            .english
            .iter()
            .map(|(code, (word, count))| format!("{word}\t{code}\t{count}\n"))
            .collect();
        match WordList::parse(&tsv) {
            Ok(list) => self.english_list = Some(list),
            Err(error) => tracing::warn!(%error, "个人英文词表重建失败"),
        }
    }

    /// 個人英文詞條數。
    pub fn english_count(&self) -> usize {
        self.english.len()
    }

    pub(super) fn save_english_to(&mut self, path: &Path) -> Result<(), LearningError> {
        let mut rows: Vec<&(String, u32)> = self.english.values().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        write_atomic(path, |file| {
            writeln!(file, "# 青简个人英文词：词\\t次数")?;
            for (word, count) in rows {
                writeln!(file, "{word}\t{count}")?;
            }
            Ok(())
        })?;
        self.english_dirty = false;
        Ok(())
    }

    /// 按輸入串記的選擇檔案與詞頻檔案同目錄。
    pub(super) fn choices_path(frequency_path: &Path) -> PathBuf {
        frequency_path.with_file_name(USER_CHOICES_FILE)
    }

    /// 從 `輸入串\t詞\t次數` 讀按輸入串記的選擇，返回跳過的壞行數。
    pub(super) fn load_choices(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let Some((input, text, count)) = parse_counted_pair(line) else {
                skipped += 1;
                continue;
            };
            if count > 0 {
                self.choices
                    .entry(input.to_owned())
                    .or_default()
                    .insert(text.to_owned(), count);
            }
        }
        skipped
    }

    pub(super) fn save_choices_to(&mut self, path: &Path) -> Result<(), LearningError> {
        let mut rows: Vec<(&String, &String, &u32)> = self
            .choices
            .iter()
            .flat_map(|(input, texts)| texts.iter().map(move |(text, count)| (input, text, count)))
            .collect();
        rows.sort_by(|a, b| {
            a.0.cmp(b.0)
                .then_with(|| b.2.cmp(a.2))
                .then_with(|| a.1.cmp(b.1))
        });
        write_atomic(path, |file| {
            writeln!(file, "# 青简按输入串记的选择：输入串\t词\t次数")?;
            for (input, text, count) in rows {
                writeln!(file, "{input}\t{text}\t{count}")?;
            }
            Ok(())
        })?;
        self.choices_dirty = false;
        Ok(())
    }

    /// 個人敲錯表與詞頻檔案同目錄。
    pub(super) fn typos_path(frequency_path: &Path) -> PathBuf {
        frequency_path.with_file_name(USER_TYPOS_FILE)
    }

    /// 從 `敲的\t要的\t次數` 讀個人敲錯表，返回跳過的壞行數。
    pub(super) fn load_typos(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let Some((typed, intended, count)) = parse_counted_pair(line) else {
                skipped += 1;
                continue;
            };
            if count > 0 {
                self.typos
                    .entry(typed.to_owned())
                    .or_default()
                    .insert(intended.to_owned(), count);
            }
        }
        skipped
    }

    pub(super) fn save_typos_to(&mut self, path: &Path) -> Result<(), LearningError> {
        let mut rows: Vec<(&String, &String, &u32)> = self
            .typos
            .iter()
            .flat_map(|(typed, intended)| {
                intended
                    .iter()
                    .map(move |(syllable, count)| (typed, syllable, count))
            })
            .collect();
        rows.sort_by(|a, b| {
            a.0.cmp(b.0)
                .then_with(|| b.2.cmp(a.2))
                .then_with(|| a.1.cmp(b.1))
        });
        write_atomic(path, |file| {
            writeln!(file, "# 青简个人敲错表：敲的\t要的\t次数")?;
            for (typed, intended, count) in rows {
                writeln!(file, "{typed}\t{intended}\t{count}")?;
            }
            Ok(())
        })?;
        self.typos_dirty = false;
        Ok(())
    }

    /// 個人敲錯表條數。
    pub fn typo_count_total(&self) -> usize {
        self.typos.values().map(HashMap::len).sum()
    }

    /// 按輸入串記的選擇條數。
    pub fn choice_count(&self) -> usize {
        self.choices.values().map(HashMap::len).sum()
    }

    /// 所有按輸入串記的計數減半，去掉減到零的。
    pub(super) fn decay_choices(&mut self) {
        for texts in self.choices.values_mut() {
            texts.retain(|_, count| {
                *count /= 2;
                *count > 0
            });
        }
        self.choices.retain(|_, texts| !texts.is_empty());
    }

    /// 使用者詞檔案與詞頻檔案同目錄。
    pub(super) fn words_path(frequency_path: &Path) -> PathBuf {
        frequency_path.with_file_name(USER_WORDS_FILE)
    }

    /// 個人 n-gram 檔案與詞頻檔案同目錄。
    pub(super) fn ngram_path(frequency_path: &Path) -> PathBuf {
        frequency_path.with_file_name(USER_NGRAM_FILE)
    }

    /// 個人 n-gram 裡不同的轉移條數（二元對 + 三元條）。
    pub fn ngram_transition_count(&self) -> usize {
        self.ngram.transition_count()
    }

    pub(super) fn save_ngram_to(&mut self, path: &Path) -> Result<(), LearningError> {
        let tsv = self.ngram.to_tsv();
        write_atomic(path, |file| {
            writeln!(file, "# 青简个人 n-gram：前词\t后词\t次数，句首用 <s>")?;
            file.write_all(tsv.as_bytes())
        })?;
        self.ngram_dirty = false;
        Ok(())
    }

    /// 從主詞庫同款 TSV 讀使用者詞，返回跳過的壞行數。
    pub(super) fn load_words(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let mut fields = line.split('\t');
            let (Some(text), Some(pinyin)) = (fields.next(), fields.next()) else {
                skipped += 1;
                continue;
            };
            if text.is_empty() || pinyin.trim().is_empty() {
                skipped += 1;
                continue;
            }
            self.words.insert(text.to_owned(), pinyin.trim().to_owned());
        }
        self.rebuild_words();
        skipped
    }

    /// 使用者詞很少，每次變化整個重建小詞庫即可。
    pub(super) fn rebuild_words(&mut self) {
        if self.words.is_empty() {
            self.user_dictionary = None;
            return;
        }
        let tsv: String = self
            .words
            .iter()
            .map(|(text, pinyin)| format!("{text}\t{pinyin}\t{USER_WORD_FREQUENCY}\n"))
            .collect();
        match Dictionary::parse(&tsv) {
            Ok(dictionary) => self.user_dictionary = Some(dictionary),
            Err(error) => tracing::warn!(%error, "用户词库重建失败"),
        }
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    pub(super) fn save_words_to(&mut self, path: &Path) -> Result<(), LearningError> {
        write_atomic(path, |file| {
            writeln!(file, "# 青简用户词：词\\t拼音\\t词频，与主词库同格式")?;
            for (text, pinyin) in &self.words {
                writeln!(file, "{text}\t{pinyin}\t{USER_WORD_FREQUENCY}")?;
            }
            Ok(())
        })?;
        self.words_dirty = false;
        Ok(())
    }
}
