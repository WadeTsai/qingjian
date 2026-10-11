//! 上屏：譯詞標註、按候選消耗緩衝區、對齊音節、學習與撤銷、自動造詞。

use super::alignment::Alignment;
use super::annotation::AnnotationReport;
use super::input_log::{InputLogEntry, InputLogger, InputSource};
use super::learning::Learner;
use super::query::EnglishTail;
use super::{
    AUTO_WORD_MAX_CHARS, AUTO_WORD_THRESHOLD, AUTO_WORD_THRESHOLD_SAME_BUFFER,
    EXPLICIT_TRANSITION_WEIGHT, Engine, choice_key, segment_longest_prefix,
};
use crate::candidate::{Candidate, CandidateKind, CandidateList, Language};
use crate::correction::typo;
use crate::{parser, sentence};
use qingjian_dictionary::Dictionary;
use std::time::Instant;

mod chain;
mod last;
mod transition;

pub(super) use chain::CommitChain;
pub use last::LastCommit;
pub use transition::Transition;

impl Engine {
    /// 給候選補上譯文。與 [`Self::query`] 分開呼叫，平臺層可以先畫候選再補畫譯文。
    pub fn annotate(&self, list: &mut CandidateList) -> AnnotationReport {
        let start = Instant::now();
        let mut hits = 0;
        for candidate in &mut list.items {
            let mut text = candidate.text.as_str();
            let traditional_map = self.traditional_map.borrow();
            if self.traditional
                && let Some(simp) = traditional_map.get(text)
            {
                text = simp.as_str();
            }
            candidate.translation = match candidate.kind {
                CandidateKind::Custom(_) => None,
                // 英文候選按敲的大小寫顯示（Company / COMPANY），釋義表鍵是小寫
                CandidateKind::English => self.english_translator.translate(text).or_else(|| {
                    self.english_translator
                        .translate(&text.to_ascii_lowercase())
                }),
                _ => self.translator.translate(text).map(|mut translation| {
                    self.mark_fresh(&mut translation);
                    translation
                }),
            };
            hits += usize::from(candidate.translation.is_some());
        }
        AnnotationReport {
            total: list.items.len(),
            hits,
            elapsed: start.elapsed(),
        }
    }

    /// 上屏：記入學習，從緩衝區消耗掉該候選對應的拼音，返回要提交給應用的文本。
    ///
    /// 上屏候選的譯文而不是候選本身（殼裡修飾鍵 + 數字）：學習、拼音消耗都和選了這個候選一樣，
    /// 返回第 `sense` 條釋義的譯文（0 是第一條，日文不帶注音）。候選沒有那麼多條釋義時不動，返回 `None`。
    pub fn commit_translation(&mut self, candidate: &Candidate, sense: usize) -> Option<String> {
        let text = candidate
            .translation
            .as_ref()
            .and_then(|t| t.senses().get(sense))
            .map(|s| s.text.clone())?;
        self.commit_with(candidate, InputSource::Translation, Some(sense));
        Some(text)
    }

    /// 候選比輸入短時（`kaifazhe` 選了 開發），剩餘拼音留在緩衝區，殼應接著 [`Self::query`]。
    /// 候選的最後一個音節比輸入長時（`kaif` 選了 開發），把輸入吃完。
    pub fn commit(&mut self, candidate: &Candidate) -> String {
        self.commit_with(candidate, InputSource::from(candidate.kind), None)
    }

    /// [`Self::commit`] 的內部形式：`source` 寫進輸入日誌（上屏譯詞時不是候選本身），
    /// `used_sense` 是直接打出去的那條譯詞的序號（詞彙記錄裡算「用過」）。
    pub(super) fn commit_with(
        &mut self,
        candidate: &Candidate,
        source: InputSource,
        used_sense: Option<usize>,
    ) -> String {
        let traditional_text = candidate.text.clone();
        let mut candidate_owned = candidate.clone();
        if self.traditional
            && let Some(simp) = self.traditional_map.borrow().get(&candidate_owned.text)
        {
            candidate_owned.text = simp.clone();
        }
        let candidate = &candidate_owned;
        // 整句不是一個詞，不記詞頻；按路徑上的詞逐條記轉移（喂個人 n-gram），路徑要在拼音消耗前重算
        let sentence_words = (candidate.kind == CandidateKind::Sentence)
            .then(|| self.sentence_words(candidate))
            .flatten();
        // 下面每條路都可能改學習資料，格子候選的排序跟著變
        self.forget_span_cache();
        // 一段拼音裡的第一個詞：記下整段的學習鍵，整段分幾次選完時合起來看（見 [`Self::finish_buffer`]）；
        // `split` 表示這次上屏接在同一段拼音裡前一次上屏之後
        let split = self.chain.same_buffer();
        if !split {
            let (_, key) = self.whole_scope();
            self.chain.begin_buffer(key);
        }
        let mut typos = Vec::new();
        let (consumed, input) = match candidate.kind {
            CandidateKind::Chinese => {
                self.learner.record(candidate);
                let (consumed, input) = self.consumed_by(candidate);
                self.learner.record_choice(&input, &candidate.text);
                typos = self.accepted_typos(candidate);
                (consumed, input)
            }
            // 形碼的候選沒有音節，對不上拼音；編碼是整段一起敲的，上屏吃掉整個作用域
            CandidateKind::Code => {
                self.learner.record(candidate);
                let (consumed, input) = self.whole_scope();
                self.learner.record_choice(&input, &candidate.text);
                (consumed, input)
            }
            // 英文詞帶出的 emoji 沒有音節，和英文詞一樣對應整段作用域
            CandidateKind::Emoji if candidate.syllables.is_empty() => self.whole_scope(),
            CandidateKind::Sentence => {
                typos = self.accepted_typos(candidate);
                self.consumed_by(candidate)
            }
            // emoji 按它對應詞的音節消耗拼音，不記學習
            CandidateKind::Emoji => self.consumed_by(candidate),
            // 雲端詞是針對整段作用域要的（拼音可能有錯，按音節對不上），上屏吃掉整段；詞庫裡沒有的記成使用者詞
            CandidateKind::Cloud => {
                if let Some(syllables) = self.learned_syllables(candidate) {
                    let learned = Candidate {
                        syllables,
                        ..candidate.clone()
                    };
                    if !self.knows_word(&learned) {
                        self.learner.learn_word(&learned.text, &learned.syllables);
                    }
                }
                self.learner.record(candidate);
                let (consumed, input) = self.whole_scope();
                self.learner.record_choice(&input, &candidate.text);
                (consumed, input)
            }
            // 模型直接生成的整句沒有音節對齊（`yongdockerbushuhenfangbian` 的 `docker` 不是拼音），
            // 按音節消耗拼音對不上，吃掉整段作用域；也因此沒法拆成詞記學習
            CandidateKind::Generated => self.whole_scope(),
            // 英文詞與快捷候選對應整段作用域；選中的英文詞記次數並進個人英文詞表，下次同樣的字首它靠前
            CandidateKind::English | CandidateKind::Shortcut | CandidateKind::Custom(_) => {
                if candidate.kind == CandidateKind::English {
                    self.learner.record(candidate);
                    self.learner.learn_english(&candidate.text);
                }
                self.whole_scope()
            }
        };
        self.apply_retraction(&input, &candidate.text);
        self.recording.clear();
        for (typed, intended) in &typos {
            tracing::debug!(typed, intended, "记录敲错");
            self.learner.record_typo(typed, intended);
        }
        let mut keys =
            self.composition.scope()[..consumed.min(self.composition.scope().len())].to_owned();
        // 輸入日誌按實際敲鍵原樣記：輔碼態把觸發鍵與碼段接在拼音後面（"kaifa;kf"），
        // replay 逐鍵重喂時不需要任何特殊邏輯
        if let Some(code) = &self.aux_code {
            keys.push(self.aux_code_key);
            keys.push_str(code);
        }
        let log_id = self.log_commit(&keys, &candidate.text, source);
        self.meter_commit(&candidate.text, source, false);
        // 上屏帶譯詞的中文候選：那一刻使用者看著這條譯詞，記進詞彙（英文候選的中文釋義不是學習語言，不記）
        if !self.private
            && candidate.kind != CandidateKind::English
            && let Some(translation) = &candidate.translation
        {
            for (index, sense) in translation.senses().iter().enumerate() {
                self.vocabulary.record_commit(
                    translation.language,
                    &sense.text,
                    used_sense == Some(index),
                );
            }
        }
        // 詞庫裡有、釋義表裡沒有的詞：交給釋義兜底在後臺問雲端，寫進個人釋義表，下次就有譯詞；私密輸入中不問
        if matches!(
            candidate.kind,
            CandidateKind::Chinese | CandidateKind::Cloud | CandidateKind::Code
        ) && self.gloss_filler.is_enabled()
            && !self.private
            && self.translator.language() != Language::Chinese
            && self.translator.translate(&candidate.text).is_none()
        {
            self.gloss_filler
                .request(self.translator.language(), &candidate.text);
        }
        self.composition.drain_prefix(consumed);
        // 上屏即收尾：碼段清空、回初始態（數字鍵與「標點先上屏」都走這裡）
        self.aux_code = None;
        let buffer_left = !self.composition.is_empty();
        match candidate.kind {
            CandidateKind::Chinese | CandidateKind::Cloud | CandidateKind::Code => {
                // 形碼沒有音節，`record_word` 裡按音節數做的整段造詞自然不會觸發
                self.record_word(
                    &candidate.text,
                    &candidate.syllables,
                    EXPLICIT_TRANSITION_WEIGHT,
                    true,
                    buffer_left,
                );
            }
            CandidateKind::Sentence => match sentence_words {
                Some(words) => {
                    // 句末的英文詞（我想學好rust 的 rust）記進個人英文詞表，和英文候選上屏一樣
                    if let Some(word) = words.last().filter(|w| is_english_word(w)) {
                        self.learner.learn_english(&word.text);
                    }
                    // 緊接著同一段拼音裡自選的詞（`jidiaole` 選了 擠，剩下的 掉了 走整句）：接縫是使用者自己定的，
                    // 第一個詞的轉移按自選記雙份。不參與兩詞造詞：我 + 的… 這種接縫太常見、轉移計數早就夠了，
                    // 會把 我的 一類造成使用者詞；整段合成詞由 [`Self::finish_buffer`] 管
                    let junction = self.chain.same_buffer() && self.chain.previous().is_some();
                    let last = words.len() - 1;
                    for (index, word) in words.iter().enumerate() {
                        let times = if index == 0 && junction {
                            EXPLICIT_TRANSITION_WEIGHT
                        } else {
                            1
                        };
                        self.record_word(
                            &word.text,
                            &word.syllables,
                            times,
                            false,
                            buffer_left || index < last,
                        );
                    }
                }
                None => self.chain.reset(),
            },
            CandidateKind::English
            | CandidateKind::Shortcut
            | CandidateKind::Custom(_)
            | CandidateKind::Emoji
            | CandidateKind::Generated => self.chain.reset(),
        }
        // 一次整句上屏裡的幾個詞不算分段選，只有這段拼音經過至少兩次上屏才合起來看
        let phrase = if split && !buffer_left {
            self.finish_buffer()
        } else {
            None
        };
        self.punctuation.note_committed(&candidate.text);
        self.history.record(&candidate.text);
        let learned = matches!(
            candidate.kind,
            CandidateKind::Chinese
                | CandidateKind::Code
                | CandidateKind::Cloud
                | CandidateKind::Sentence
        );
        let commit = if learned {
            LastCommit {
                text: candidate.text.clone(),
                chars: traditional_text.chars().count(),
                input,
                chosen: matches!(
                    candidate.kind,
                    CandidateKind::Chinese | CandidateKind::Code | CandidateKind::Cloud
                )
                .then(|| candidate.text.clone()),
                transitions: std::mem::take(&mut self.recording),
                typos,
                erased: 0,
                log_id,
                phrase,
            }
        } else {
            let mut plain = LastCommit::plain(&candidate.text);
            plain.chars = traditional_text.chars().count();
            plain
        };
        self.remember_commit(commit);
        traditional_text
    }

    /// 一段拼音分幾次選完了（`jidiaole` 先選 擠、剩下的走整句 掉了）：這幾個詞合起來就是使用者對這段拼音的答案。
    /// 記一次「整段拼音 → 合成詞」的選擇；選到 [`AUTO_WORD_THRESHOLD_SAME_BUFFER`] 次、詞庫裡沒有、
    /// 不超過 [`AUTO_WORD_MAX_CHARS`] 字就造成使用者詞，下次整段打出來它直接排第一。返回記下的選擇，撤銷時退回。
    pub(super) fn finish_buffer(&mut self) -> Option<(String, String)> {
        let words = self.chain.buffer_words();
        if words.len() < 2 {
            return None;
        }
        let text: String = words.iter().map(|(t, _)| t.as_str()).collect();
        let syllables: Vec<String> = words.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
        let chars = text.chars().count();
        let key = self.chain.buffer_key().to_owned();
        if key.is_empty() || chars > AUTO_WORD_MAX_CHARS || chars != syllables.len() {
            return None;
        }
        self.learner.record_choice(&key, &text);
        let candidate = Candidate {
            text,
            kind: CandidateKind::Chinese,
            syllables,
            reading: None,
            translation: None,
            aux_code: None,
        };
        if !self.knows_word(&candidate)
            && self.learner.choice_weight(&key, &candidate.text) >= AUTO_WORD_THRESHOLD_SAME_BUFFER
        {
            tracing::debug!(text = %candidate.text, "整段拼音分次选完，自动造词");
            self.learner
                .learn_word(&candidate.text, &candidate.syllables);
        }
        Some((key, candidate.text))
    }

    /// 最近刪掉的上屏裡有一次是同一段拼音（或它的字首）、這次卻選了別的詞：把那次記的學習退回去。
    /// 刪掉幾個詞再從頭重打是常事（「沃德 書」刪掉重打成「我的 書」），所以往前找最近一次刪乾淨的同段拼音；
    /// 重打後選的還是同一個詞就不算選錯，只把那條記錄丟掉。這次的拼音與刪掉的哪次都對不上時，刪掉的那些當作在改別處，全忘掉。
    pub(super) fn apply_retraction(&mut self, input: &str, text: &str) {
        let Some(index) = self
            .recent_commits
            .iter()
            .rposition(|c| c.is_erased() && c.same_input(input))
        else {
            // 刪掉的那次拼音與這次相近但不同（wode 刪了重打 wodi）：重打了鍵，記進輸入日誌給敲錯糾正當樣本；學習不動
            if let Some(erased) = self
                .recent_commits
                .iter()
                .rev()
                .find(|c| c.is_erased() && is_retyped(&c.input, input))
            {
                self.logger.record(InputLogEntry::Retype {
                    before: erased.input.clone(),
                    after: input.to_owned(),
                    of: erased.log_id,
                });
            }
            self.recent_commits.retain(|c| !c.is_erased());
            return;
        };
        let last = self.recent_commits.remove(index);
        if last.text == text {
            return;
        }
        tracing::debug!(retracted = %last.text, chosen = %text, "上次选错了，撤销它的学习");
        self.logger.record(InputLogEntry::Retract {
            of: last.log_id,
            text: last.text.clone(),
            chosen: text.to_owned(),
        });
        if let Some(chosen) = &last.chosen {
            self.learner.unrecord(chosen);
            self.learner.unrecord_choice(&last.input, chosen);
        }
        for transition in &last.transitions {
            self.learner.unrecord_transition(
                transition.context(),
                &transition.word,
                transition.times,
            );
        }
        for (typed, intended) in &last.typos {
            self.learner.unrecord_typo(typed, intended);
        }
        if let Some((key, phrase)) = &last.phrase {
            self.learner.unrecord_choice(key, phrase);
        }
    }

    /// 這個候選上屏算接受了哪些音節級敲錯：詞圖裡靠敲錯變體對上的音節，加上整段糾錯落在的那個音節（吃到了編輯處才算）。
    /// 雙拼不記（鍵與全拼對不上）。
    pub(super) fn accepted_typos(&self, candidate: &Candidate) -> Vec<(String, String)> {
        let keys = self.composition.scope();
        if self.decode(keys).is_some() {
            return Vec::new();
        }
        match self.active_correction(keys) {
            Some(c) => {
                let alignment = self.align(&c.corrected, &candidate.syllables);
                let mut typos = alignment.typos;
                typos.extend(c.typo_pair(alignment.consumed));
                typos
            }
            None => self.align(keys, &candidate.syllables).typos,
        }
    }

    /// 整句候選對應的詞序列：重算一次整句轉換，文本對得上才算（對不上說明候選來自別處，不記）。
    /// 末尾是英文詞的整句（我想學好rust）先按頭段試，英文詞作為最後一個詞（音節就是敲的字母）。
    pub(super) fn sentence_words(
        &self,
        candidate: &Candidate,
    ) -> Option<Vec<sentence::SentenceWord>> {
        let scope = self.composition.scope();
        if self.decode(scope).is_none()
            && let Some(tail) = self.split_english_tail(scope)
            && let Some(words) = self.mixed_words(scope, &tail, &candidate.text)
        {
            return Some(words);
        }
        let conversion = match (self.decode(scope), self.active_correction(scope)) {
            (Some(decoded), _) => {
                self.convert_sentence(&decoded.segmentation()?.patterns(), true)?
            }
            (None, Some(c)) => self.convert_sentence(&c.segmentation.patterns(), false)?,
            (None, None) => {
                let (segmentations, _) = segment_longest_prefix(scope).ok()?;
                self.convert_sentence(&segmentations.first()?.patterns(), true)?
            }
        };
        (conversion.text == candidate.text).then_some(conversion.words)
    }

    /// 頭段拼音的轉換加上英文尾段，與 `text` 對得上時的詞序列。
    fn mixed_words(
        &self,
        scope: &str,
        tail: &EnglishTail,
        text: &str,
    ) -> Option<Vec<sentence::SentenceWord>> {
        let segmentations = parser::segment(&scope[..tail.head_len]).ok()?;
        let mut conversion = self.convert_sentence(&segmentations.first()?.patterns(), true)?;
        conversion.text.push_str(&tail.word);
        if conversion.text != text {
            return None;
        }
        conversion.words.push(sentence::SentenceWord {
            text: tail.word.clone(),
            syllables: vec![scope[tail.head_len..].to_owned()],
            placeholder: false,
        });
        Some(conversion.words)
    }

    /// 候選消耗多少作用域位元組，以及按輸入串記學習用的鍵（候選覆蓋的那段全拼字母）。
    /// 糾錯生效時按糾正後的拼音算，再按那處編輯換算回原串；雙拼按解出的全拼算，再換算回鍵數。
    pub(super) fn consumed_by(&self, candidate: &Candidate) -> (usize, String) {
        let keys = self.composition.scope();
        if let Some(decoded) = self.decode(keys) {
            let pinyin_len = self.align(decoded.pinyin(), &candidate.syllables).consumed;
            return (
                decoded.keys_for(pinyin_len),
                choice_key(decoded.pinyin(), pinyin_len),
            );
        }
        let consumed = match self.active_correction(keys) {
            Some(c) => c
                .edit
                .to_original(self.align(&c.corrected, &candidate.syllables).consumed)
                .min(keys.len()),
            None => self.align(keys, &candidate.syllables).consumed,
        };
        (consumed, choice_key(keys, consumed))
    }

    /// 候選的音節逐個對到輸入上：每個音節按原樣 / 規範寫法、模糊音或敲錯變體（長的在前）、沒打完的字首消耗輸入，
    /// 先找能把**每個音節都對上**的對齊（帶回溯：`pingyin` 對 拼音 時 `pin` 原樣只吃三個字母會剩下 `gyin`，
    /// 退回來按敲錯變體 `ping` → `pin` 吃四個），找不到才退回逐個貪心對、對不上的地方停下。
    pub(super) fn align(&self, input: &str, syllables: &[String]) -> Alignment {
        let mut typos = Vec::new();
        self.align_full(input, 0, syllables, &mut typos)
            .unwrap_or_else(|| self.align_greedy(input, syllables))
    }

    /// 從 `pos` 起把剩下的音節全對上的第一種對齊（按每步的優先順序深度優先）；對不上返回 `None`。
    fn align_full(
        &self,
        input: &str,
        pos: usize,
        syllables: &[String],
        typos: &mut Vec<(String, String)>,
    ) -> Option<Alignment> {
        let Some((syllable, remaining)) = syllables.split_first() else {
            return Some(Alignment {
                consumed: pos,
                typos: typos.clone(),
            });
        };
        let (rest, start) = self.rest_at(input, pos);
        for (len, typo) in self.syllable_steps(rest, syllable) {
            // 消耗完輸入後還有音節沒對：不算全對上（候選比敲的長）
            if start + len == input.len() && !remaining.is_empty() {
                continue;
            }
            if typo {
                typos.push((rest[..len].to_owned(), syllable.clone()));
            }
            let found = self.align_full(input, start + len, remaining, typos);
            if typo {
                typos.pop();
            }
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// 逐個貪心對：每個音節取第一種能對上的消耗，都對不上就取公共字首，公共字首也沒有就停。
    fn align_greedy(&self, input: &str, syllables: &[String]) -> Alignment {
        let mut alignment = Alignment::default();
        let mut pos = 0;
        for syllable in syllables {
            let (rest, start) = self.rest_at(input, pos);
            if let Some((len, typo)) = self.syllable_steps(rest, syllable).into_iter().next() {
                if typo {
                    alignment
                        .typos
                        .push((rest[..len].to_owned(), syllable.clone()));
                }
                pos = start + len;
                continue;
            }
            let common = syllable
                .bytes()
                .zip(rest.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            if common == 0 {
                break;
            }
            pos = start + common;
        }
        alignment.consumed = pos;
        alignment
    }

    /// `pos` 處這個音節能看到的輸入段（到下一個 `'` 為止）與它的起點（跳過開頭的 `'`）。
    fn rest_at<'a>(&self, input: &'a str, pos: usize) -> (&'a str, usize) {
        let start = if pos > 0 && input[pos..].starts_with('\'') {
            pos + 1
        } else {
            pos
        };
        let rest = &input[start..];
        let rest = &rest[..rest.find('\'').unwrap_or(rest.len())];
        (rest, start)
    }

    /// 一個音節可以怎麼消耗輸入段 `rest`：(消耗位元組數, 是否靠敲錯變體)，按優先順序排：
    /// 原樣或規範寫法（`lue` / `lve`）、模糊音或敲錯變體（長的在前）、整段是這個音節沒打完的字首。
    fn syllable_steps(&self, rest: &str, syllable: &str) -> Vec<(usize, bool)> {
        let mut steps = Vec::new();
        let exact = rest.get(..syllable.len()).is_some_and(|typed| {
            typed == syllable
                || qingjian_dictionary::canonical_syllable(typed)
                    == qingjian_dictionary::canonical_syllable(syllable)
        });
        if exact {
            steps.push((syllable.len(), false));
        }
        // 沒打完排在變體前面：`shijia` 選 時間 是 jian 沒敲完，不是把 jian 敲成了 jia
        if !rest.is_empty() && rest.len() < syllable.len() && syllable.starts_with(rest) {
            steps.push((rest.len(), false));
        }
        for len in (1..=rest.len().min(parser::MAX_SYLLABLE_LEN)).rev() {
            if exact && len == syllable.len() {
                continue;
            }
            let typed = &rest[..len];
            if self.fuzzy.is_variant(typed, syllable) {
                steps.push((len, false));
            } else if typo::is_variant(typed, syllable) {
                steps.push((len, true));
            }
        }
        steps
    }

    /// 整段作用域對應的候選（英文詞、雲端詞、快捷候選）：吃掉全部鍵，學習鍵是整段全拼。
    pub(super) fn whole_scope(&self) -> (usize, String) {
        let keys = self.composition.scope();
        let input = match self.decode(keys) {
            Some(decoded) => choice_key(decoded.pinyin(), decoded.pinyin().len()),
            None => choice_key(keys, keys.len()),
        };
        (keys.len(), input)
    }

    /// 一箇中文詞上屏了：記 `times` 份轉移、推進鏈；`auto_word` 為真（使用者自己選的詞）時，
    /// 緊接著上一個詞、合起來詞庫裡沒有、且這條接續記夠次數還自動造詞。
    pub(super) fn record_word(
        &mut self,
        text: &str,
        syllables: &[String],
        times: u32,
        auto_word: bool,
        buffer_left: bool,
    ) {
        self.learner
            .record_transition(self.chain.context(), text, times);
        self.recording
            .push(Transition::new(self.chain.context(), text, times));
        if auto_word {
            let threshold = if self.chain.same_buffer() {
                AUTO_WORD_THRESHOLD_SAME_BUFFER
            } else {
                AUTO_WORD_THRESHOLD
            };
            self.try_auto_word(text, syllables, threshold);
        }
        self.chain.advance(text, syllables, buffer_left);
    }

    /// 上一個詞 + 這個詞合成使用者詞的條件見 [`AUTO_WORD_THRESHOLD`]。
    pub(super) fn try_auto_word(&mut self, text: &str, syllables: &[String], threshold: u32) {
        let Some(previous) = self.chain.previous().map(str::to_owned) else {
            return;
        };
        let joined = format!("{previous}{text}");
        let chars = joined.chars().count();
        let mut joined_syllables = self.chain.previous_syllables().to_vec();
        joined_syllables.extend(syllables.iter().cloned());
        if chars > AUTO_WORD_MAX_CHARS || chars != joined_syllables.len() {
            return;
        }
        // 這條轉移剛記過，計數已含本次；閾值按「選了幾次」算，計數是按份記的
        let seen = self
            .learner
            .user_ngram()
            .map_or(0, |b| b.pair(Some(&previous), text));
        if seen < threshold * EXPLICIT_TRANSITION_WEIGHT {
            return;
        }
        let candidate = Candidate {
            text: joined,
            kind: CandidateKind::Chinese,
            syllables: joined_syllables,
            reading: None,
            translation: None,
            aux_code: None,
        };
        if self.knows_word(&candidate) {
            return;
        }
        tracing::debug!(text = %candidate.text, "自动造词");
        self.learner
            .learn_word(&candidate.text, &candidate.syllables);
    }

    /// 主詞庫或使用者詞裡是否已有這個詞（同音節）。
    pub(super) fn knows_word(&self, candidate: &Candidate) -> bool {
        let syllables: Vec<&str> = candidate.syllables.iter().map(String::as_str).collect();
        if syllables.is_empty() {
            return true;
        }
        let known = |dictionary: &Dictionary| {
            dictionary
                .lookup(&syllables, false)
                .iter()
                .any(|hit| hit.exact && hit.text == candidate.text)
        };
        self.all_dictionaries().into_iter().any(known)
    }
}

/// 整句路徑上的詞是英文詞（`woxiangxuehaorust` 的 rust）：不是佔位音節、全是字母。
fn is_english_word(word: &sentence::SentenceWord) -> bool {
    !word.placeholder && !word.text.is_empty() && word.text.bytes().all(|b| b.is_ascii_alphabetic())
}

/// 刪掉重打的鍵串算不算「同一段拼音打錯了」：都夠長、不相等、編輯距離不超過 [`RETYPE_MAX_EDITS`]。
/// 差得更多的是在改別的內容，不算。
fn is_retyped(before: &str, after: &str) -> bool {
    before.len() >= RETYPE_MIN_LETTERS
        && after.len() >= RETYPE_MIN_LETTERS
        && before != after
        && edit_distance(before, after) <= RETYPE_MAX_EDITS
}

/// 跨上屏重打的鍵串最少幾個字母才算數。
const RETYPE_MIN_LETTERS: usize = 3;

/// 跨上屏重打最多差幾處編輯。
const RETYPE_MAX_EDITS: usize = 2;

/// Levenshtein 編輯距離（按位元組，鍵串都是 ASCII）。
fn edit_distance(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}
