use super::*;

/// 「拼音頭 + 英文尾」的切法：`woxiangxuehaorust` 切成頭 `woxiangxuehao` 與尾 rust。
/// 見 [`Engine::split_english_tail`]。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EnglishTail {
    /// 頭段（拼音）佔作用域開頭多少位元組。
    pub head_len: usize,

    /// 尾段對應的英文詞，按詞表裡的寫法（`api` → API）。
    pub word: String,

    /// 整段字母也能讀成拼音（`database` → da ta ba se，`woxiangxuehaorust` → … ru s… t… 簡拼）：兩種讀法要比分，
    /// 見 [`Engine::mixed_beats_plain`]；整段根本切不成拼音的（`wodeid` 的 i、`woyongvim` 的 v）直接按英文讀。
    pub competes: bool,

    /// 這個英文詞的 log 機率（按詞表詞頻估），比分用。
    pub log_prob: f64,
}

/// wordfreq 的 Zipf 頻率（詞表裡 ×1000 存）換成 log 機率：Zipf z 是每十億詞裡 10^z 次，即 log P = (z − 9)·ln 10。
/// 沒有詞頻的（個人表裡的詞、`kubectl` 這類技術詞）按 Zipf 3（百萬分之一）算。
pub(crate) fn english_log_prob(frequency: Option<u32>) -> f64 {
    let zipf = frequency
        .map(|f| f64::from(f) / 1000.0)
        .filter(|z| *z > 0.0)
        .unwrap_or(ENGLISH_ZIPF_FLOOR)
        .max(ENGLISH_ZIPF_FLOOR);
    (zipf - 9.0) * std::f64::consts::LN_10
}

impl Engine {
    /// 整段輸入的末尾是不是一個英文詞：`woxiangxuehaorust` → 我想學好 + rust。
    ///
    /// 尾段要在英文詞表裡（個人表或隨包表），頭段要能切成每個音節都完整的拼音。
    /// 尾段自己就是完整拼音的（`database`、`fan`）要至少 [`MIN_PINYIN_LIKE_TAIL_LETTERS`] 個字母；
    /// 兩個字母的尾段只認縮寫詞（ID / TV / OK）和個人表裡的詞：`to` / `it` 這類太容易撞上簡拼。
    /// 同時滿足的取最長的尾段（`wodedatabase` 取 database 不取 base）。雙拼、帶 `'` 的輸入不切；
    /// 整段本身是英文詞（`agent`）、拼音不像話時糾錯能糾通（`shiide` → 是的）或有英文補全（`releas` → release）的也不切，
    /// 那幾條路本來就排第一。
    /// 切出來只說明「可以這麼讀」，與拼音讀法誰排前面看 `competes` 與比分。
    pub(crate) fn split_english_tail(&self, scope: &str) -> Option<EnglishTail> {
        if self.shuangpin.is_some()
            || scope.len() < MIN_ENGLISH_TAIL_HEAD_LETTERS + MIN_ENGLISH_TAIL_LETTERS
            || !scope.bytes().all(|b| b.is_ascii_lowercase())
        {
            return None;
        }
        let lists = self.english_lists();
        if lists.is_empty() {
            return None;
        }
        // 整段是隨包表裡的英文詞（`agent`）：那條路本來就排第一，不切。
        // 個人表不算：打不出來時原樣上屏會把整串失敗的拼音（`woxiangxuexirust`）學成「英文詞」，
        // 再拿它把混輸這條路堵掉就是死迴圈——越打不出來學得越多，越學越打不出來
        if self
            .english
            .as_ref()
            .is_some_and(|words| words.get(scope).is_some())
        {
            return None;
        }
        let full = parser::segment(scope).ok();
        let unlikely = correction::unlikely_pinyin(full.as_ref().and_then(|s| s.first()), "")
            || correction::trailing_single_letter(full.as_ref().and_then(|s| s.first()));
        if full.is_none() || unlikely {
            // 拼寫糾錯能把整段糾成通順的拼音（`shiide` → 是的，`yingagi` → 應該）：那是敲錯，不是英文
            if self.active_correction(scope).is_some() {
                return None;
            }
            if scope.len() >= MIN_COMPLETION_LETTERS
                && lists
                    .iter()
                    .any(|words| !words.complete(scope, 1).is_empty())
            {
                return None;
            }
        }
        let personal = self.learner.user_english();
        let longest = scope.len() - MIN_ENGLISH_TAIL_HEAD_LETTERS;
        (MIN_ENGLISH_TAIL_LETTERS..=longest).rev().find_map(|len| {
            let head_len = scope.len() - len;
            let tail = &scope[head_len..];
            let words = lists.iter().find(|words| words.get(tail).is_some())?;
            let word = words.get(tail)?;
            let acronym = word.bytes().any(|b| b.is_ascii_uppercase());
            let known = personal.is_some_and(|words| words.get(tail).is_some());
            if len == MIN_ENGLISH_TAIL_LETTERS && !acronym && !known {
                return None;
            }
            if parser::is_fully_segmentable(tail) && len < MIN_PINYIN_LIKE_TAIL_LETTERS {
                return None;
            }
            if !parser::is_fully_segmentable(&scope[..head_len]) {
                return None;
            }
            Some(EnglishTail {
                head_len,
                word: word.to_owned(),
                competes: full.is_some(),
                // 詞頻只問隨包表：個人表存的是使用次數，當 Zipf×1000 讀會一律塌到兜底值
                //（`English` 用過 2 次 → Zipf 0.002 → 兜底 3.0，真實 5.19，白丟 5 nat），
                // 等於一個英文詞用過一次就更難打出來。個人表裡獨有的詞（`kubectl`）本來就走兜底
                log_prob: english_log_prob(
                    self.english
                        .as_ref()
                        .and_then(|words| words.frequency(tail)),
                ),
            })
        })
    }

    /// 整段也能讀成拼音時兩種讀法比分：頭段整句的得分加英文詞的 log 機率、扣掉切到英文的代價，高過整段按拼音讀的整句就按英文讀。
    /// 拼音讀法末尾的單字母也讀（`huoz` → 或者，不是 或 + 丟掉 z），兩邊覆蓋同樣多的字母才公平。
    /// `wodedatabase`：我的 + database 贏過 我的大塔巴瑟；`womenqubeijing`：我們去北京 贏過 我們去 + Beijing；
    /// `taida`：太大 贏過 他 + Ida；`huoz`：或者 贏過 和 + Oz。
    ///
    /// 兩邊比的是 `static_score - penalty`（靜態語言模型 + 敲錯代價），**不含個人 n-gram 與使用者加分**：
    /// 頭段只覆蓋一部分音節，整段讀法多出來的那幾個音節會白拿一份按長度累積的個人加成。
    /// `woxiangxuexirust` 上實測頭段拿 +6.9、整段拿 +21.2，個人模型越肥差距越大——
    /// 使用者打得越多混輸越打不出來。這是「哪種輸入方式」的判斷，本來也該由靜態模型定，不歸個人頻次管。
    pub(super) fn mixed_beats_plain(&self, scope: &str, tail: &EnglishTail) -> bool {
        let convert = |text: &str, whole: bool| {
            let segmentations = parser::segment(text).ok()?;
            self.convert_sentence_with(&segmentations.first()?.patterns(), true, whole)
        };
        let (Some(head), Some(plain)) = (
            convert(&scope[..tail.head_len], false),
            convert(scope, true),
        ) else {
            return false;
        };
        if head.has_placeholder() {
            return false;
        }
        let comparable = |c: &Conversion| c.static_score - c.penalty;
        let mixed = comparable(&head) + tail.log_prob - ENGLISH_SWITCH_PENALTY;
        let plain_score = comparable(&plain);
        tracing::debug!(
            word = %tail.word,
            head = %head.text, head = comparable(&head),
            plain = %plain.text, plain = plain_score,
            tail_log_prob = tail.log_prob, mixed,
            wins = mixed > plain_score,
            "混输比分"
        );
        mixed > plain_score
    }

    /// 頭段拼音轉成的漢字加上英文尾段：候選的音節是頭段的全拼音節加上敲的尾段字母（上屏按它們消耗拼音）。
    /// 頭段有佔位音節的不出。
    pub(super) fn mixed_sentence(
        &self,
        head: &Segmentation,
        tail: &EnglishTail,
        typos: bool,
    ) -> Option<Candidate> {
        let conversion = self.convert_sentence(&head.patterns(), typos)?;
        if conversion.has_placeholder() {
            return None;
        }
        let typed = &self.composition.scope()[tail.head_len..];
        let mut syllables = conversion.syllables;
        syllables.push(typed.to_owned());
        Some(Candidate {
            text: format!("{}{}", conversion.text, tail.word),
            kind: CandidateKind::Sentence,
            syllables,
            reading: None,
            translation: None,
            aux_code: None,
        })
    }
}
