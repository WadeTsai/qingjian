//! 注入與開關：詞庫、模糊音、雙拼、翻譯 / 學習 / 聯想等 trait 實現的掛接，以及相應的只讀訪問。

use super::aux_code::is_valid_aux_code_key;
use super::*;
use crate::engine::decoded::EngineDecoded;

impl Engine {
    /// 設定中文模式的標點轉換。
    pub fn set_full_width_punctuation(&mut self, enabled: bool) {
        self.full_width_punctuation = enabled;
    }

    /// 原子更新自定義短語，非法規則保持舊值。
    pub fn set_custom_phrases(&mut self, phrases: Vec<crate::CustomPhrase>) -> Result<(), String> {
        crate::custom_phrase::validate_phrases(&phrases)?;
        self.custom_phrases = phrases;
        Ok(())
    }

    /// 設雙拼方案，`None` 回到全拼。糾錯快取按作用域記而作用域的含義變了，一併清掉。
    pub fn set_shuangpin(&mut self, scheme: Option<Scheme>) {
        self.shuangpin = scheme;
        *self.correction_cache.borrow_mut() = None;
    }

    pub fn shuangpin(&self) -> Option<Scheme> {
        self.shuangpin
    }
    /// 換雙拼 preedit 顯示形式：開著時顯示原始按鍵（如 `ljse`），關著（預設）展開成全拼（`lan'se`）。
    pub fn set_shuangpin_raw_preedit(&mut self, enabled: bool) {
        self.shuangpin_raw_preedit = enabled;
    }

    pub fn shuangpin_raw_preedit(&self) -> bool {
        self.shuangpin_raw_preedit
    }

    /// 掛上輔碼碼錶（使用者匯入的與隨包的筆畫表）。照 [`Self::set_extra_dictionaries`] 的模式：殼按目錄與配置裝配。
    pub fn with_aux_codes(mut self, tables: Vec<Arc<dyn AuxCodeLookup>>) -> Self {
        self.aux_codes = tables;
        self
    }

    /// 執行時換碼錶（匯入 / 移除 / 開關之後）；順帶退出輔碼態，免得篩的是一張已經不在的表。
    pub fn set_aux_codes(&mut self, tables: Vec<Arc<dyn AuxCodeLookup>>) {
        self.aux_codes = tables;
        self.aux_code = None;
    }

    pub fn aux_codes(&self) -> &[Arc<dyn AuxCodeLookup>] {
        &self.aux_codes
    }

    /// 換輔碼觸發鍵（配置項 `[general] aux_code_key`）。非法的鍵（含當前翻頁鍵 `page_keys`，
    /// 配置項 `[general] page_keys`）退回預設 `;`。
    pub fn set_aux_code_key(&mut self, key: char, page_keys: (char, char)) {
        self.aux_code_key = if is_valid_aux_code_key(key, page_keys) {
            key
        } else {
            DEFAULT_AUX_CODE_KEY
        };
    }

    /// 換「碼刪空後留在輔碼態」開關（配置項 `[general] aux_code_keep_empty`，預設開）。
    pub fn set_aux_keep_empty(&mut self, keep: bool) {
        self.aux_keep_empty = keep;
    }

    /// 換輔碼總開關（配置項 `[aux_code] enabled`，預設關）。關掉時輔碼整線關：觸發鍵不進輔碼態，
    /// 純拼音態也不逐候查首條碼；開著但沒有碼錶（`aux_codes` 為空）同樣不進輔碼態。
    pub fn set_aux_enabled(&mut self, enabled: bool) {
        self.aux_enabled = enabled;
    }

    /// 換「候選上顯示碼」開關（配置項 `[general] aux_code_show`，預設關）：純拼音態掛不掛首條碼。
    pub fn set_aux_show(&mut self, show: bool) {
        self.aux_show = show;
    }

    /// 學習開關（`[general] learning`）：關掉後不再記詞頻、使用者詞、個人 n-gram 與敲錯表，已學的照常參與排序；
    /// 私密輸入是另一個獨立的開關（[`Self::set_private`]）。
    pub fn set_learning(&mut self, enabled: bool) {
        self.learner.set_disabled(!enabled);
    }

    /// 設定是否啟用注音模式。開啟後鍵盤輸入按當前排列（[`Self::set_zhuyin_layout`]）解析。
    pub fn set_zhuyin_mode(&mut self, on: bool) {
        self.zhuyin = on.then_some(self.zhuyin_layout);
        self.forget_span_cache();
    }

    /// 換注音鍵盤排列（`[general] zhuyin_layout`）：大千 / 倚天。注音關著時只記下，開啟時按它解。
    pub fn set_zhuyin_layout(&mut self, layout: ZhuyinLayout) {
        self.zhuyin_layout = layout;
        if self.zhuyin.is_some() {
            self.zhuyin = Some(layout);
            self.forget_span_cache();
        }
    }

    /// 當前的注音鍵盤排列（注音關著時也有值）。
    pub fn zhuyin_layout(&self) -> ZhuyinLayout {
        self.zhuyin_layout
    }

    /// 注音模式下這個鍵是不是符號 / 聲調鍵（殼據此把數字、標點收進緩衝區）；注音關著時一律不是，空格另算。
    pub fn is_zhuyin_key(&self, c: char) -> bool {
        self.zhuyin.is_some_and(|layout| layout.is_key(c))
    }

    /// 設定是否啟用繁體輸出模式。
    pub fn set_traditional_mode(&mut self, on: bool) {
        self.traditional = on;
        if on && self.opencc.is_none() {
            match ferrous_opencc::OpenCC::from_config(ferrous_opencc::config::BuiltinConfig::S2tw) {
                Ok(opencc) => self.opencc = Some(opencc),
                Err(error) => tracing::warn!(%error, "繁體轉換器初始化失敗，候選仍是簡體"),
            }
        }
    }

    /// 目前是否處於注音模式。
    pub fn is_zhuyin_mode(&self) -> bool {
        self.zhuyin.is_some()
    }

    /// 換形碼碼錶（五筆），`None` 回到拼音的諸方案。編碼與拼音是兩套鍵，糾錯快取一併清掉。
    pub fn set_code_table(&mut self, table: Option<CodeTable>) {
        self.code = table;
        *self.correction_cache.borrow_mut() = None;
        self.forget_span_cache();
    }

    /// 當前的形碼碼錶；`None` 表示走拼音（全拼 / 雙拼 / 注音）。
    pub fn code_table(&self) -> Option<&CodeTable> {
        self.code.as_ref()
    }

    /// 是否處在形碼方案下。
    pub fn is_code_mode(&self) -> bool {
        self.code.is_some()
    }

    /// 拼音側參不參與查詢。形碼開著時把它關掉就是「只用形碼」（`[general] scheme = "none"`）；
    /// 兩邊都開是混輸，見 [`Self::set_code_table`] 與 [`Self::query_mixed`]。
    pub fn set_phonetic(&mut self, on: bool) {
        self.phonetic = on;
        *self.correction_cache.borrow_mut() = None;
        self.forget_span_cache();
    }

    pub fn is_phonetic(&self) -> bool {
        self.phonetic
    }

    /// 判斷注音模式下目前是否還需要輸入聲調。
    /// 供殼（平臺層）用來判斷空白鍵是應該進緩衝區作為聲調，還是直接用來選詞。
    pub fn zhuyin_needs_tone(&self) -> bool {
        let Some(layout) = self.zhuyin else {
            return false;
        };
        let raw = self.composition.text();
        if raw.is_empty() {
            return false;
        }
        let decoded = crate::zhuyin::decode(layout, raw);
        if let Some(last) = decoded.units().last() {
            !last.complete && last.pinyin != "'"
        } else {
            false
        }
    }

    /// 組句中敲 `;` 是否該進緩衝區：微軟 / 搜狗雙拼裡它是 ing 的韻母鍵，只在末尾有落單的聲母時收，
    /// 其他時候仍是標點。問字模式（`?x`）看的是字首之後的部分。
    pub fn takes_semicolon(&self) -> bool {
        let body = self
            .modes()
            .question_body(self.composition.scope(), self.zhuyin);
        self.shuangpin
            .filter(|scheme| scheme.uses_semicolon())
            .is_some_and(|scheme| scheme.decode(body).pending_initial())
    }

    /// 只用形碼：碼錶掛著、拼音側關著。
    pub(super) fn code_only(&self) -> bool {
        self.code.is_some() && !self.phonetic
    }

    /// 混輸：碼錶掛著、拼音側也開著。
    pub(super) fn mixed(&self) -> bool {
        self.code.is_some() && self.phonetic
    }

    /// 有效的模式鍵：只用形碼時所有字母都是字根鍵，只剩 `?` 開頭的問字；
    /// 雙拼與混輸下小寫字母各有用處，換成大寫字母。
    pub(super) fn modes(&self) -> ModeKeys {
        if self.code_only() {
            self.modes.letterless()
        } else if self.shuangpin.is_some() || self.mixed() {
            self.modes.shifted()
        } else {
            self.modes
        }
    }

    /// 緩衝區為空時敲的大寫字母該不該進表示式 / 問字模式：只在雙拼或混輸下、且是模式鍵的大寫時。
    /// 殼只在中文模式、Caps 滅時問。
    pub fn takes_mode_letter(&self, c: char) -> bool {
        let modes = self.modes();
        (self.shuangpin.is_some() || self.mixed())
            && self.zhuyin.is_none()
            && (c == modes.expression || c == modes.question)
    }

    /// 緩衝區為空時敲 `?` 該不該進問字模式（配置 `[shortcut] question_mark`）：殼據此決定問號是入口還是標點。
    pub fn takes_question_mark(&self) -> bool {
        self.modes().question_mark
    }

    /// 雙拼開著時把一段鍵解成全拼；全拼下為 `None`，呼叫方原樣用鍵。
    pub(super) fn decode(&self, keys: &str) -> Option<EngineDecoded> {
        if let Some(layout) = self.zhuyin {
            Some(EngineDecoded::Zhuyin(crate::zhuyin::decode(layout, keys)))
        } else {
            self.shuangpin
                .map(|scheme| EngineDecoded::Shuangpin(scheme.decode(keys)))
        }
    }

    /// 游標後剩餘拼音的顯示形式：雙拼先解碼；能切就按音節用 `'` 連上，切不動就原樣。
    /// 只用形碼時剩餘段是編碼，原樣顯示。
    pub(super) fn marked_rest(&self, rest: &str) -> String {
        if self.code_only() || (self.shuangpin.is_some() && self.shuangpin_raw_preedit) {
            return rest.to_owned();
        }
        match self.decode(rest) {
            Some(decoded) => decoded.marked(),
            None => marked_rest(rest),
        }
    }

    pub fn with_emoji(mut self, table: EmojiTable) -> Self {
        self.emoji = Some(table);
        self
    }

    pub fn with_fuzzy(mut self, rules: FuzzyRules) -> Self {
        self.fuzzy = rules;
        self
    }

    /// 換模糊音規則：格子快取裡的代價隨寫法變，一起作廢。
    pub fn set_fuzzy(&mut self, rules: FuzzyRules) {
        if self.fuzzy != rules {
            self.forget_span_cache();
        }
        self.fuzzy = rules;
    }

    pub fn fuzzy(&self) -> FuzzyRules {
        self.fuzzy
    }

    pub fn with_predictor(mut self, predictor: Box<dyn Predictor>) -> Self {
        self.predictor = predictor;
        self
    }

    /// 執行時換掉 Predictor（選單開關雲聯想 / 配置熱載入）；正在等的聯想一併作廢。
    pub fn set_predictor(&mut self, predictor: Box<dyn Predictor>) {
        self.cancel_prediction();
        self.predictor = predictor;
    }

    /// 掛上同步的整句重打分器（字級 Transformer，查詢裡當場打分，評測用）。`weight` 是神經分的權重 λ，
    /// `margin` 是參與重排的路徑分門檻（nat），`context` 是給模型看的前文字元數；
    /// `None` 用預設 [`NEURAL_WEIGHT`] / [`NEURAL_MARGIN`] / [`RESCORE_CONTEXT_CHARS`]。
    pub fn with_sentence_scorer(
        mut self,
        scorer: Box<dyn SentenceScorer>,
        weight: Option<f64>,
        margin: Option<f64>,
        context: Option<usize>,
    ) -> Self {
        self.sentence_scorer = Some(scorer);
        self.rescorer = None;
        self.set_neural_parameters(weight, margin, context);
        self
    }

    /// 掛上非同步的整句重打分器：打分在後臺執行緒，查詢不等它，殼在停頓後 [`Self::request_rescoring`]、
    /// 結果到了 [`Self::poll_rescoring`] 後再查一次。引數同 [`Self::with_sentence_scorer`]。
    pub fn with_async_sentence_scorer(
        mut self,
        scorer: Box<dyn SentenceScorer>,
        weight: Option<f64>,
        margin: Option<f64>,
        context: Option<usize>,
    ) -> Self {
        self.set_async_sentence_scorer(Some(scorer));
        self.set_neural_parameters(weight, margin, context);
        self
    }

    /// 執行時換 / 卸非同步重打分器（殼裡模型在後臺載入完才接上，配置關掉就卸）。
    pub fn set_async_sentence_scorer(&mut self, scorer: Option<Box<dyn SentenceScorer>>) {
        self.sentence_scorer = None;
        self.rescorer = scorer.map(super::rescoring::RescoreWorker::spawn);
        *self.neural_cache.borrow_mut() = super::rescoring::NeuralCache::default();
        self.forget_span_cache();
    }

    /// 換一組個人 n-gram 插值引數（回放調參用）；整句格子快取作廢。
    pub fn set_interpolation(&mut self, interpolation: Interpolation) {
        self.interpolation = interpolation;
        self.forget_span_cache();
    }

    pub fn interpolation(&self) -> Interpolation {
        self.interpolation
    }

    /// 換一組敲錯糾正代價（回放調參用）；整句格子快取與糾錯快取作廢。
    pub fn set_typo_costs(&mut self, costs: TypoCosts) {
        self.typo_costs = costs;
        *self.correction_cache.borrow_mut() = None;
        self.forget_span_cache();
    }

    pub fn typo_costs(&self) -> TypoCosts {
        self.typo_costs
    }

    /// 整句轉換與詞級排序用的個人部分：學習器的個人 n-gram 配上當前插值引數。
    pub(super) fn personal(&self) -> Personal<'_> {
        Personal {
            ngram: self.learner.user_ngram(),
            interpolation: self.interpolation,
        }
    }

    /// 神經分的權重 λ（0 到 1）。
    pub fn set_neural_weight(&mut self, weight: f64) {
        self.neural_weight = weight.clamp(0.0, 1.0);
        self.forget_span_cache();
    }

    fn set_neural_parameters(
        &mut self,
        weight: Option<f64>,
        margin: Option<f64>,
        context: Option<usize>,
    ) {
        self.neural_weight = weight.unwrap_or(NEURAL_WEIGHT).clamp(0.0, 1.0);
        self.neural_margin = margin.unwrap_or(NEURAL_MARGIN).max(0.0);
        self.neural_context = context.unwrap_or(RESCORE_CONTEXT_CHARS);
        self.forget_span_cache();
    }

    pub fn with_language_model(mut self, model: Box<dyn LanguageModel>) -> Self {
        self.language_model = model;
        self
    }

    /// 靜態語言模型（沒接就是 [`NoLanguageModel`]）：評測工具拿它按 [`crate::sentence::segment_text`] 切漢字文本。
    pub fn language_model(&self) -> &dyn LanguageModel {
        &*self.language_model
    }

    pub fn history(&self) -> &InputHistory {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut InputHistory {
        &mut self.history
    }

    /// 進入 / 離開英文模式。英文模式下 [`Self::query`] 只給英文詞表的候選，回車與空格仍由殼原樣上屏敲的字母，
    /// 不發雲聯想，也不把原樣上屏記成「不糾這個串」。
    pub fn set_english_mode(&mut self, on: bool) {
        self.english_mode = on;
    }

    pub fn english_mode(&self) -> bool {
        self.english_mode
    }

    pub fn with_english(mut self, words: WordList) -> Self {
        self.english = Some(words);
        self
    }

    pub fn with_translator(mut self, translator: Box<dyn Translator>) -> Self {
        self.translator = translator;
        self
    }

    /// 執行時換學習語言的釋義表。
    /// 接英文候選用的釋義表（英→中）。
    pub fn with_english_translator(mut self, translator: Box<dyn Translator>) -> Self {
        self.english_translator = translator;
        self
    }

    pub fn set_translator(&mut self, translator: Box<dyn Translator>) {
        self.translator = translator;
    }

    pub fn with_mode_keys(mut self, keys: ModeKeys) -> Self {
        self.modes = keys.sanitized();
        self
    }

    /// 非法組合（相同、或不是 v / u / i）整個退回預設。
    pub fn set_mode_keys(&mut self, keys: ModeKeys) {
        self.modes = keys.sanitized();
    }

    pub fn mode_keys(&self) -> ModeKeys {
        self.modes
    }

    /// 候選後面配不配 emoji（配置 `[general] emoji`，預設開）。
    pub fn set_emoji_candidates(&mut self, on: bool) {
        self.emoji_candidates = on;
    }

    pub fn emoji_candidates(&self) -> bool {
        self.emoji_candidates
    }

    /// 中文模式下給不給英文詞與補全（配置 `[general] english_in_chinese`，預設開）；英文模式不受影響。
    pub fn set_english_in_chinese(&mut self, on: bool) {
        self.english_in_chinese = on;
    }

    pub fn english_in_chinese(&self) -> bool {
        self.english_in_chinese
    }

    /// 中英混輸裡中文候選是否總排在英文詞前面（配置 `[general] chinese_first`，預設關）。
    /// 關著時拼音「不像話」的輸入英文詞排第一（`hello` 先英文再 荷蘭咯）；開了英文詞固定第二。
    pub fn set_chinese_first(&mut self, on: bool) {
        self.chinese_first = on;
    }

    pub fn chinese_first(&self) -> bool {
        self.chinese_first
    }

    /// 中文模式下 Shift+字母是否進組句緩衝區（配置 `[general] shift_letter`，預設關）。
    /// 開著時大寫按小寫參與匹配、原樣上屏時還原，`Cpan` 與 `cpan` 一樣出「C盤」；
    /// 關著時殼直接把大寫字母交給應用，進這裡的字母就按它自己的樣子匹配。
    pub fn set_shift_letter_compose(&mut self, on: bool) {
        self.shift_letter_compose = on;
    }

    pub fn shift_letter_compose(&self) -> bool {
        self.shift_letter_compose
    }

    pub fn with_learner(mut self, learner: Box<dyn Learner>) -> Self {
        self.learner.replace(learner);
        self.forget_span_cache();
        self
    }

    pub fn with_input_logger(mut self, logger: Box<dyn InputLogger>) -> Self {
        self.logger.replace(logger);
        self
    }

    /// 執行時換輸入日誌的落盤方（開關、清空之後）。舊的先 flush。
    pub fn set_input_logger(&mut self, logger: Box<dyn InputLogger>) {
        self.logger.flush();
        self.logger.replace(logger);
    }

    pub fn input_logger_mut(&mut self) -> &mut dyn InputLogger {
        self.logger.inner_mut()
    }

    pub fn with_usage_meter(mut self, meter: Box<dyn UsageMeter>) -> Self {
        self.meter = meter;
        self
    }

    /// 輸入統計的彙總（偏好設定「統計」頁）。
    pub fn usage_summary(&self) -> UsageSummary {
        self.meter.summary()
    }

    pub fn with_vocabulary_tracker(mut self, tracker: Box<dyn VocabularyTracker>) -> Self {
        self.vocabulary = tracker;
        self
    }

    pub fn with_gloss_filler(mut self, filler: Box<dyn GlossFiller>) -> Self {
        self.gloss_filler = filler;
        self
    }

    /// 執行時換釋義兜底（隨雲聯想開關）。
    pub fn set_gloss_filler(&mut self, filler: Box<dyn GlossFiller>) {
        self.gloss_filler = filler;
    }

    pub fn dictionary(&self) -> &Dictionary {
        &self.dictionary
    }

    /// 換掉全部附加詞庫（匯入、移除、開關之後）。格子快取隨之作廢。
    pub fn set_extra_dictionaries(&mut self, dictionaries: Vec<Dictionary>) {
        self.extra_dictionaries = dictionaries;
        self.forget_span_cache();
    }

    pub fn extra_dictionaries(&self) -> &[Dictionary] {
        &self.extra_dictionaries
    }

    /// 查詞用的全部詞庫：主詞庫、附加詞庫、使用者詞。
    pub(super) fn all_dictionaries(&self) -> Vec<&Dictionary> {
        let mut all = Vec::with_capacity(self.extra_dictionaries.len() + 2);
        all.push(&self.dictionary);
        all.extend(self.extra_dictionaries.iter());
        if let Some(user) = self.learner.user_words() {
            all.push(user);
        }
        all
    }

    /// 全部詞庫的詞頻之和，詞頻歸一化成機率時用。
    pub(super) fn total_frequency(&self) -> u64 {
        self.all_dictionaries()
            .iter()
            .map(|d| d.total_frequency())
            .sum()
    }

    pub fn learner(&self) -> &dyn Learner {
        self.learner.inner()
    }

    /// 拿到可變的 Learner 就當它要改：格子快取一起作廢。
    pub fn learner_mut(&mut self) -> &mut dyn Learner {
        self.forget_span_cache();
        self.learner.inner_mut()
    }

    pub fn learning_language(&self) -> Language {
        self.translator.language()
    }
}
