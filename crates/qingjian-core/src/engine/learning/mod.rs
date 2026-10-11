//! 學習與統計的掛鉤：釋義兜底回填、詞彙曝光、輸入統計、輸入日誌、刪候選、定時落盤。

use super::Engine;
use super::input_log::{CommitEntry, InputLogEntry, InputLogger, InputSource, LOGGED_CANDIDATES};
use super::statistics::Usage;
use super::vocabulary::{FRESH_UNTIL, VocabularySummary};
use crate::candidate::{Candidate, CandidateKind, Translation};
use crate::sentence;

mod forgotten;
mod learner;
mod muted;

pub use forgotten::Forgotten;
pub use learner::{Learner, NoLearner};
pub(super) use muted::MutedLearner;

impl Engine {
    /// 取回釋義兜底寫好的釋義，記進譯者（個人釋義表）；返回學了幾條。殼定時調，不阻塞。
    /// 結果的語言與當前學習語言對不上（中途切過語言）就丟。
    pub fn poll_glosses(&mut self) -> usize {
        let filled = self.gloss_filler.poll();
        let mut learned = 0;
        for gloss in filled {
            if gloss.translation.language == self.translator.language() {
                tracing::debug!(word = %gloss.word, "释义兜底写入个人释义表");
                self.translator.learn(&gloss.word, gloss.translation);
                learned += 1;
            }
        }
        learned
    }

    /// 當前學習語言的詞彙彙總（偏好設定「統計」頁）。
    pub fn vocabulary_summary(&self) -> VocabularySummary {
        self.vocabulary.summary(self.translator.language())
    }

    /// 殼畫完候選視窗後告知當前頁上的候選：頁上的譯詞在使用者上屏那一刻記成「看到過」（[`super::vocabulary::VocabularyTracker`]）。
    /// 每次重畫都換掉上一頁，逐鍵重新整理時一閃而過的候選不算；視窗收起時傳空。
    pub fn note_displayed<'a>(&mut self, candidates: impl IntoIterator<Item = &'a Candidate>) {
        self.displayed.clear();
        for candidate in candidates {
            if candidate.kind == CandidateKind::English {
                continue;
            }
            let Some(translation) = &candidate.translation else {
                continue;
            };
            for sense in translation.senses() {
                let key = (translation.language, sense.text.clone());
                if !self.displayed.contains(&key) {
                    self.displayed.push(key);
                }
            }
        }
    }

    /// 上屏了：當前頁上的譯詞都算看到過一輪。
    pub(super) fn record_exposures(&mut self) {
        if self.private {
            // 私密輸入中的候選頁不能汙染詞彙記錄；同時丟掉暫存頁，避免之後恢復普通輸入時補記。
            self.displayed.clear();
            return;
        }
        for (language, word) in std::mem::take(&mut self.displayed) {
            self.vocabulary.record_exposure(language, &word);
        }
    }

    /// 按詞彙記錄給譯詞標生詞：看到的輪次不到 [`FRESH_UNTIL`] 的算。
    pub(super) fn mark_fresh(&self, translation: &mut Translation) {
        let language = translation.language;
        for sense in translation.senses_mut() {
            sense.fresh = self.vocabulary.exposures(language, &sense.text) < FRESH_UNTIL;
        }
    }

    /// 往輸入統計記一次上屏：漢字與英文詞按文字數，中文詞數按來源定（選一個詞算一個，整句按語言模型切出來的詞數，
    /// 切不了就按字數）。`english_word` 是原樣上屏的字母串算不算一個英文詞（拼音回車不算）。
    pub(super) fn meter_commit(&mut self, text: &str, source: InputSource, english_word: bool) {
        let mut usage = Usage::of_text(text);
        usage.words = match source {
            InputSource::Word | InputSource::Cloud => 1,
            InputSource::Sentence | InputSource::CloudSentence => {
                sentence::segment_text(text, &*self.language_model).map_or(usage.hanzi, |clauses| {
                    clauses.iter().map(|words| words.len() as u64).sum()
                })
            }
            InputSource::English
            | InputSource::Custom
            | InputSource::Shortcut
            | InputSource::Emoji
            | InputSource::Raw
            | InputSource::Translation => 0,
        };
        if source == InputSource::Raw && !english_word {
            usage.english_words = 0;
        }
        self.meter.record(usage);
    }

    /// 往輸入日誌記一次上屏。`keys` 是這次消耗掉的原始鍵，`index` 從上一次查詢的候選裡找。
    /// 攢著的直通字元先寫出去（順序要對）；這段組句裡退格過且最終鍵串變了，緊跟著記一條 `retype`。
    pub(super) fn log_commit(&mut self, keys: &str, text: &str, source: InputSource) -> u64 {
        self.record_exposures();
        self.flush_passthrough();
        self.log_sequence += 1;
        let snapshot = self.last_query.borrow().clone().unwrap_or_default();
        let index = snapshot.candidates.iter().position(|c| c == text);
        let ms = self
            .composition_started
            .map_or(0, |started| started.elapsed().as_millis() as u64);
        self.logger.record(InputLogEntry::Commit(CommitEntry {
            id: self.log_sequence,
            scope: snapshot.scope,
            keys: keys.to_owned(),
            pinyin: snapshot.pinyin,
            corrected: snapshot.corrected,
            text: text.to_owned(),
            source,
            index,
            top: snapshot
                .candidates
                .into_iter()
                .take(LOGGED_CANDIDATES)
                .collect(),
            scheme: self.scheme_key(),
            english: self.english_mode,
            rescored: snapshot.rescored,
            app: self.application.clone(),
            pages: self.page_turns,
            ms,
        }));
        self.committed_since_break = true;
        self.page_turns = 0;
        if let Some(before) = self.retype_snapshot.take() {
            let after = self.composition.scope();
            if before != after {
                self.logger.record(InputLogEntry::Retype {
                    before,
                    after: after.to_owned(),
                    of: self.log_sequence,
                });
            }
        }
        self.log_sequence
    }

    /// 使用者要求刪掉一個候選（修飾鍵 + 數字）：中文詞與雲端詞交給 Learner 刪使用者詞、清學習；英文詞刪個人英文詞；
    /// 整句、快捷候選、emoji 沒什麼可刪。刪完快取作廢，它也不再當下一個詞的上文。
    pub fn forget(&mut self, candidate: &Candidate) -> Forgotten {
        let mut candidate_owned = candidate.clone();
        if self.traditional
            && let Some(simp) = self.traditional_map.borrow().get(&candidate_owned.text)
        {
            candidate_owned.text = simp.clone();
        }
        let candidate = &candidate_owned;
        let forgotten = match candidate.kind {
            CandidateKind::Chinese | CandidateKind::Code | CandidateKind::Cloud => {
                self.learner.forget(&candidate.text)
            }
            CandidateKind::English => Forgotten {
                user_word: self.learner.forget_english(&candidate.text),
                learning: false,
            },
            CandidateKind::Sentence
            | CandidateKind::Shortcut
            | CandidateKind::Custom(_)
            | CandidateKind::Emoji
            | CandidateKind::Generated => Forgotten::default(),
        };
        if !forgotten.is_nothing() {
            self.forget_span_cache();
            *self.correction_cache.borrow_mut() = None;
            if self.chain.mentions(&candidate.text) {
                self.chain.reset();
            }
            self.recent_commits.retain(|c| c.text != candidate.text);
            tracing::debug!(text = %candidate.text, ?forgotten, "删除候选");
        }
        forgotten
    }

    /// 把學習資料與輸入日誌落盤。殼在停用輸入法時調，啟用期間也可以定時調（程序被殺時少丟）：
    /// 落盤不改學習狀態，所以不像 [`Self::learner_mut`] 那樣作廢格子快取。
    pub fn flush_learning(&mut self) {
        self.learner.flush();
        self.logger.flush();
        self.meter.flush();
        self.vocabulary.flush();
        self.translator.flush();
    }

    /// 詞庫 / 使用者詞 / 學習資料變了，整句格子候選全部作廢。
    pub(super) fn forget_span_cache(&self) {
        self.span_cache.borrow_mut().clear();
    }
}
