//! 聯想：候選之外的非同步補充。
//!
//! [`Predictor`] 由殼注入（網路實現在 `qingjian-predict`，Core 永遠不聯網），Engine 負責裁剪上下文、
//! 編號請求、校驗雲端詞的拼音、丟棄過期結果。聯想**不參與排序、不阻塞輸入**；
//! 雲端詞到了只補進候選視窗第一頁末尾幾格（[`crate::CandidateLayout`]），前面的本地候選不挪。
//!
//! Engine 這一側的實現：組句中發請求、收結果校驗、Tab 接受整句；翻譯選中文字也走這裡。

mod cloud_word;
mod fuzzy;
mod kind;
mod policy;
mod predictor;
mod question;
mod request;
mod response;
mod script;
mod surrounding_text;

pub use cloud_word::CloudWord;
pub use fuzzy::{mismatch_count, tolerance};
pub use kind::PredictionKind;
pub use policy::PredictionPolicy;
pub use predictor::{NoPredictor, Predictor};
pub use question::restates_question;
pub use request::PredictionRequest;
pub use response::Prediction;
pub use script::translation_target;
pub use surrounding_text::SurroundingText;

use super::*;

impl Engine {
    /// 是否接了會聯想的 Predictor；殼據此決定要不要起輪詢定時器、畫雲朵標識。
    pub fn prediction_enabled(&self) -> bool {
        self.predictor.is_enabled()
    }

    pub fn prediction_policy(&self) -> PredictionPolicy {
        self.predictor.policy()
    }

    /// 發一次聯想請求，返回序號；沒接 Predictor、私密輸入中或拼音太短時不發，返回 `None`。
    ///
    /// 要的是「當前作用域拼音對應的詞」和整句補全；`candidates` 是本地候選，只取前幾個當提示。
    /// `surrounding` 是應用給的游標前後文本，在這裡按觀察視窗裁剪，殼給得再多也只發這麼多；
    /// 應用給不出上下文就只靠拼音，本地輸入歷史不可靠、不用。上屏之後不聯想：沒有拼音約束的下文聯想只是噪音。
    pub fn request_prediction(
        &mut self,
        surrounding: Option<SurroundingText>,
        candidates: &[Candidate],
    ) -> Option<u64> {
        if !self.predictor.is_enabled() || self.private {
            return None;
        }
        // 不發也要換序號：正在飛的舊結果對應的是上一個輸入狀態，回來了也不能顯示
        self.prediction_sequence += 1;
        // 輔碼態在按碼篩詞，雲端詞沒有碼、進來只會打亂；不發請求，槽位自然收起
        if self.aux_filter().is_some() {
            return None;
        }
        let policy = self.predictor.policy();
        let (before, after) = match surrounding {
            Some(text) => (
                take_last_chars(&text.before, policy.before),
                take_first_chars(&text.after, policy.after),
            ),
            None => (String::new(), String::new()),
        };
        let scope = self.composition.scope();
        if self.english_mode
            || self.modes().is_expression(scope, self.zhuyin)
            || is_raw(scope, self.modes(), self.shuangpin, self.zhuyin)
        {
            return None;
        }
        // 問字模式：問題本身就是全部上下文，不帶應用文本、不要整句、本地沒有候選可提示
        let question = self.modes().is_question(scope, self.zhuyin);
        if question
            && shortcut::unicode_form(self.modes().question_body(scope, self.zhuyin)).is_some()
        {
            // 碼點輸入本地就能答，不問雲端
            return None;
        }
        let (kind, pinyin_source, before, after) = if question {
            (
                PredictionKind::Question,
                self.modes().question_body(scope, self.zhuyin),
                String::new(),
                String::new(),
            )
        } else {
            (PredictionKind::Compose, scope, before, after)
        };
        // 雙拼：問雲端用的是解出來的全拼，不是敲的鍵
        let decoded = self.decode(pinyin_source);
        let pinyin_source: &str = decoded.as_ref().map_or(pinyin_source, |d| d.pinyin());
        let letters = pinyin_source.chars().filter(|c| *c != '\'').count();
        if letters < MIN_PREDICTION_LETTERS {
            return None;
        }
        let (pinyin, syllables, guess, abbreviated) = match segment_longest_prefix(pinyin_source) {
            Ok((segmentations, tail)) => {
                let best = segmentations.first();
                (
                    query::join_marked(&segmentations, tail),
                    best.map_or(0, |s| s.syllables.len()),
                    self.local_guess(&segmentations),
                    best.is_some_and(mostly_abbreviated),
                )
            }
            Err(_) => (pinyin_source.to_owned(), 0, String::new(), false),
        };
        if question {
            self.last_question_guess = guess.clone();
        }
        let request = PredictionRequest {
            sequence: self.prediction_sequence,
            kind,
            before,
            after,
            pinyin,
            letters: pinyin_source.replace('\'', ""),
            syllables,
            candidates: candidates
                .iter()
                .take(PREDICTION_CANDIDATE_HINTS)
                .map(|c| c.text.clone())
                .collect(),
            guess,
            // 簡拼（半數以上音節是縮寫）不問詞：模型按聲母湊出來的大多是生造詞（複合語氣、符號對映），
            // 只問整句補全；問字模式的答案不受這條限制（答案本來就對不上問題的拼音）。
            max_items: if abbreviated && !question {
                0
            } else {
                policy.max_items
            },
            want_sentence: policy.sentence && !question,
            text: String::new(),
            target_language: String::new(),
        };
        self.last_prediction_kind = kind;
        self.last_prediction_scope = scope.to_owned();
        self.predictor.submit(request);
        Some(self.prediction_sequence)
    }

    /// 把應用裡選中的一段文字交給雲端翻譯（殼裡快捷鍵觸發）：主要是漢字就譯成學習語言，是外文（拉丁字母、假名）就譯成中文
    /// （[`translation_target`]）。雲聯想關著、私密輸入中、文字為空時不發，返回 `None`；
    /// 譯文從 [`Self::poll_prediction`] 的 `sentence` 裡出。不進學習、不動緩衝區。
    pub fn request_translation(&mut self, text: &str) -> Option<u64> {
        let text = text.trim();
        if !self.predictor.is_enabled() || self.private || text.is_empty() {
            return None;
        }
        self.prediction_sequence += 1;
        let request = PredictionRequest {
            sequence: self.prediction_sequence,
            kind: PredictionKind::Translate,
            before: String::new(),
            after: String::new(),
            pinyin: String::new(),
            letters: String::new(),
            syllables: 0,
            candidates: Vec::new(),
            guess: String::new(),
            max_items: 1,
            want_sentence: true,
            text: text.to_owned(),
            target_language: translation_target(text, self.translator.language())
                .code()
                .to_owned(),
        };
        self.last_prediction_kind = PredictionKind::Translate;
        self.predictor.submit(request);
        Some(self.prediction_sequence)
    }

    /// 作廢正在飛的聯想（使用者清空了拼音、關掉了聯想框）。
    pub fn cancel_prediction(&mut self) {
        self.prediction_sequence += 1;
    }

    /// 取回最近一次請求的結果；過期結果直接丟。沒有就緒的結果返回 `None`，不阻塞。
    /// 結果可能是空的（模型沒給出可用條目），殼據此停止等待。
    pub fn poll_prediction(&mut self) -> Option<Prediction> {
        while let Some(mut prediction) = self.predictor.poll() {
            if prediction.sequence == self.prediction_sequence {
                // 問字的答案、翻譯的譯文和敲的拼音本來就對不上，只有組句聯想的雲端詞要校驗
                if self.last_prediction_kind == PredictionKind::Question {
                    let guess = &self.last_question_guess;
                    prediction
                        .words
                        .retain(|word| !restates_question(&word.text, guess));
                } else if self.last_prediction_kind == PredictionKind::Compose
                    && !self.question_mode()
                {
                    if self.predictor.policy().slots == 0 {
                        // 使用者不要雲端詞，只留整句補全
                        prediction.words.clear();
                    } else {
                        self.validate_cloud_words(&mut prediction.words);
                    }
                    if !prediction.is_empty() {
                        // 給過使用者什麼：緊接著的上屏說明接沒接受（本地聯想能不能替代雲端的尺子）
                        self.logger.record(InputLogEntry::Prediction {
                            scope: self.last_prediction_scope.clone(),
                            words: prediction.words.iter().map(|w| w.text.clone()).collect(),
                            sentence: prediction.sentence.clone(),
                        });
                    }
                }
                if self.traditional
                    && self.last_prediction_kind != PredictionKind::Translate
                    && let Some(opencc) = &self.opencc
                {
                    for word in &mut prediction.words {
                        let traditional = opencc.convert(&word.text);
                        self.traditional_map
                            .borrow_mut()
                            .insert(traditional.clone(), word.text.clone());
                        word.text = traditional;
                    }
                    if let Some(sentence) = &mut prediction.sentence {
                        let traditional = opencc.convert(sentence);
                        self.traditional_map
                            .borrow_mut()
                            .insert(traditional.clone(), sentence.clone());
                        *sentence = traditional;
                    }
                }
                return Some(prediction);
            }
            tracing::debug!(
                sequence = prediction.sequence,
                current = self.prediction_sequence,
                "丢弃过期的联想结果"
            );
        }
        None
    }

    /// 只留下拼音對得上的雲端詞：字數等於音節數、每個音節合法、全拼與使用者敲的字母的編輯距離在容許範圍內
    /// （簡拼不算錯，允許少量錯字 / 漏字 / 多字，糾錯就靠這個）。模型偶爾會給出根本不是這個拼音的詞，這些不進候選。
    pub(super) fn validate_cloud_words(&self, words: &mut Vec<CloudWord>) {
        let decoded = self.decode(self.composition.scope());
        let typed = decoded
            .as_ref()
            .map_or(self.composition.scope(), |d| d.pinyin());
        let letters = typed.chars().filter(|c| *c != '\'').count();
        let allowed = tolerance(letters);
        words.retain(|word| {
            let fits = !word.syllables.is_empty()
                && word.text.chars().count() == word.syllables.len()
                && word.syllables.iter().all(|s| parser::is_syllable(s))
                && mismatch_count(typed, &word.syllables) <= allowed;
            if !fits {
                tracing::debug!(text = %word.text, syllables = ?word.syllables, "云端词与拼音不符，丢弃");
            }
            fits
        });
    }

    /// 使用者接受一條整句補全：作用域內的拼音作廢、句子上屏。句子沒有拼音，記不了詞頻與使用者詞，
    /// 但按語言模型把它切成詞（[`sentence::segment_text`]）逐條記進個人 n-gram，與選整句候選一樣；
    /// 標點處斷句，句尾是標點時之後的詞按句首記。整句退格刪光再重打時這些轉移一併退回。
    pub fn accept_prediction(&mut self, text: &str) -> String {
        let traditional_text = text.to_owned();
        let original_text_owned;
        let text = if self.traditional {
            original_text_owned = self
                .traditional_map
                .borrow()
                .get(text)
                .cloned()
                .unwrap_or_else(|| text.to_owned());
            &original_text_owned
        } else {
            text
        };
        let (_, input) = self.whole_scope();
        self.apply_retraction(&input, text);
        self.recording.clear();
        let keys = self.composition.scope().to_owned();
        let log_id = self.log_commit(&keys, text, InputSource::CloudSentence);
        self.meter_commit(text, InputSource::CloudSentence, false);
        self.composition.drain_scope();
        self.punctuation.note_committed(text);
        self.history.record(text);
        match sentence::segment_text(text, &*self.language_model) {
            Some(clauses) => {
                let buffer_left = !self.composition.is_empty();
                let count = clauses.len();
                for (index, words) in clauses.iter().enumerate() {
                    // 第一句接著前面上屏的詞；標點之後的各句從句首起
                    if index > 0 {
                        self.chain.reset();
                    }
                    for word in words {
                        self.record_word(word, &[], 1, false, buffer_left || index + 1 < count);
                    }
                }
                if text.chars().last().is_some_and(|c| !c.is_alphanumeric()) {
                    self.chain.reset();
                }
                tracing::debug!(clauses = count, "整句补全已记入个人 n-gram");
            }
            None => self.chain.reset(),
        }
        let commit = LastCommit {
            text: text.to_owned(),
            chars: traditional_text.chars().count(),
            input,
            chosen: None,
            transitions: std::mem::take(&mut self.recording),
            typos: Vec::new(),
            erased: 0,
            log_id,
            phrase: None,
        };
        self.remember_commit(commit);
        traditional_text
    }

    /// 雲端詞學成使用者詞時用哪套音節。模型給的讀音偶有錯（我的 → wo di），錯讀音學進去以後只會按錯讀音出來，
    /// 還會把整句候選里正確的同文本詞頂掉；使用者敲的拼音能切成與字數相同的完整音節、每個又是對應字的讀音時以敲的為準，
    /// 否則仍用模型的，但模型的讀音裡有詞庫不認「這個字讀這個音」的就不學。
    pub(super) fn learned_syllables(&self, candidate: &Candidate) -> Option<Vec<String>> {
        let chars: Vec<String> = candidate.text.chars().map(String::from).collect();
        let decoded = self.decode(self.composition.scope());
        let typed = decoded
            .as_ref()
            .map_or(self.composition.scope(), |d| d.pinyin());
        if let Ok((segmentations, "")) = segment_longest_prefix(typed)
            && let Some(best) = segmentations.first()
            && best.syllables.len() == chars.len()
            && best
                .syllables
                .iter()
                .zip(&chars)
                .all(|(s, c)| s.complete && self.accepts_reading(c, &s.text))
        {
            return Some(best.syllables.iter().map(|s| s.text.clone()).collect());
        }
        let fits = candidate.syllables.len() == chars.len()
            && candidate
                .syllables
                .iter()
                .zip(&chars)
                .all(|(s, c)| self.accepts_reading(c, s));
        fits.then(|| candidate.syllables.clone())
    }

    /// 這個字按這個讀音能不能接受：詞庫裡這個字有這個讀音；詞庫根本不認識這個字（生僻字）時沒法核對，照單全收。
    fn accepts_reading(&self, ch: &str, syllable: &str) -> bool {
        self.char_reads(ch, syllable) || !parser::SYLLABLES.iter().any(|s| self.char_reads(ch, s))
    }

    /// 詞庫（含使用者詞）裡這個字有沒有這個讀音。
    fn char_reads(&self, ch: &str, syllable: &str) -> bool {
        let pattern = [qingjian_dictionary::SyllablePattern {
            text: syllable,
            complete: true,
        }];
        self.all_dictionaries()
            .into_iter()
            .any(|d| d.lookup_exact(&pattern).iter().any(|hit| hit.text == ch))
    }
}

/// 半數以上音節是縮寫（聲母 `f` 或未打完的字首 `zho`）：這種輸入下模型按聲母湊詞基本只會給生造詞，
/// 只問整句補全。`fhyq` → 符合要求 是縮寫，`fuheyaoqiu` 不是，`nih`（兩全一縮）也不是。
fn mostly_abbreviated(segmentation: &Segmentation) -> bool {
    let total = segmentation.syllables.len();
    total > 0
        && segmentation
            .syllables
            .iter()
            .filter(|syllable| !syllable.complete)
            .count()
            * 2
            >= total
}
