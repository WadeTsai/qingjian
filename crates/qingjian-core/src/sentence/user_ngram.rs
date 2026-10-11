use foldhash::{HashMap, HashMapExt};

use super::{Context, Interpolation, MAX_USER_TRANSITIONS, SENTENCE_START};

/// 個人 n-gram：使用者上屏過的詞序列計數（二元 + 三元），隨上屏線上更新，進整句轉換與詞級排序的打分。
///
/// 二元只存 (前詞, 後詞) 計數：每個上屏的詞都帶著前詞（句首用 [`SENTENCE_START`]），
/// 所以一個詞的出現次數就是以它為後詞的計數之和，無須另存一元表。
/// 三元存 (前二詞, 前詞, 後詞) 計數，句首詞不記三元（與二元重複）。
/// 持久化格式是 TSV：三列 `前詞\t後詞\t次數` 是二元，四列 `前二詞\t前詞\t後詞\t次數` 是三元；檔案讀寫在 learning crate，這裡只做解析與序列化。
#[derive(Debug, Default, Clone)]
pub struct UserNgram {
    /// 前詞 → (後詞 → 次數)。
    pairs: HashMap<String, HashMap<String, u32>>,

    /// 前詞 → 以它為前詞的總次數（`pairs[v]` 的和），算二元條件機率的分母。
    context_totals: HashMap<String, u32>,

    /// 詞 → 出現次數（以它為後詞的計數之和）。
    word_counts: HashMap<String, u32>,

    /// 所有二元計數之和（不含句首標記作為後詞的情況，它不會出現在後詞位置）。
    total: u64,

    /// 前二詞 → (前詞 → (後詞 → 次數))。
    triples: HashMap<String, HashMap<String, HashMap<String, u32>>>,

    /// 前二詞 → (前詞 → 這對上文的總次數)，算三元條件機率的分母。
    triple_totals: HashMap<String, HashMap<String, u32>>,
}

impl UserNgram {
    /// 記一次轉移。超過上限時把所有計數減半，忘掉久遠的偏好。
    pub fn record(&mut self, context: Context<'_>, word: &str) {
        self.record_times(context, word, 1);
    }

    /// 一次記 `times` 份：使用者自己點選的轉移比整句路徑裡順帶的更可信，Engine 給它記雙份。
    pub fn record_times(&mut self, context: Context<'_>, word: &str, times: u32) {
        if word.is_empty() || times == 0 {
            return;
        }
        let previous = context.previous.unwrap_or(SENTENCE_START);
        *self
            .pairs
            .entry(previous.to_owned())
            .or_default()
            .entry(word.to_owned())
            .or_default() += times;
        *self.context_totals.entry(previous.to_owned()).or_default() += times;
        *self.word_counts.entry(word.to_owned()).or_default() += times;
        self.total += u64::from(times);
        if let Some(previous) = context.previous {
            let earlier = context.earlier.unwrap_or(SENTENCE_START);
            *self
                .triples
                .entry(earlier.to_owned())
                .or_default()
                .entry(previous.to_owned())
                .or_default()
                .entry(word.to_owned())
                .or_default() += times;
            *self
                .triple_totals
                .entry(earlier.to_owned())
                .or_default()
                .entry(previous.to_owned())
                .or_default() += times;
        }
        if self.transition_count() > MAX_USER_TRANSITIONS {
            self.decay();
        }
    }

    /// 退回 `times` 份轉移（使用者撤銷了剛才的上屏）：減到零就刪掉，沒記過的不動。
    pub fn unrecord(&mut self, context: Context<'_>, word: &str, times: u32) {
        let previous = context.previous.unwrap_or(SENTENCE_START);
        if let Some(next) = self.pairs.get_mut(previous) {
            let removed = remove_times(next, word, times);
            if next.is_empty() {
                self.pairs.remove(previous);
            }
            decrement(&mut self.context_totals, previous, removed);
            decrement(&mut self.word_counts, word, removed);
            self.total = self.total.saturating_sub(u64::from(removed));
        }
        let Some(previous) = context.previous else {
            return;
        };
        let earlier = context.earlier.unwrap_or(SENTENCE_START);
        let Some(by_previous) = self.triples.get_mut(earlier) else {
            return;
        };
        let Some(next) = by_previous.get_mut(previous) else {
            return;
        };
        let removed = remove_times(next, word, times);
        if next.is_empty() {
            by_previous.remove(previous);
        }
        if by_previous.is_empty() {
            self.triples.remove(earlier);
        }
        if let Some(totals) = self.triple_totals.get_mut(earlier) {
            decrement(totals, previous, removed);
            if totals.is_empty() {
                self.triple_totals.remove(earlier);
            }
        }
    }

    /// 忘掉一個詞：刪掉它在任何位置出現的全部轉移，重算各項合計。返回刪掉的二元計數總和。
    pub fn forget_word(&mut self, word: &str) -> u32 {
        let mut removed = 0u32;
        if let Some(next) = self.pairs.remove(word) {
            for (following, count) in next {
                removed += count;
                decrement(&mut self.word_counts, &following, count);
            }
            self.context_totals.remove(word);
        }
        let mut emptied = Vec::new();
        for (previous, next) in &mut self.pairs {
            if let Some(count) = next.remove(word) {
                removed += count;
                decrement(&mut self.context_totals, previous, count);
                if next.is_empty() {
                    emptied.push(previous.clone());
                }
            }
        }
        for previous in emptied {
            self.pairs.remove(&previous);
        }
        self.word_counts.remove(word);
        self.total = self.total.saturating_sub(u64::from(removed));
        self.forget_word_in_triples(word);
        removed
    }

    /// 三元表裡刪掉 `word` 作為前二詞、前詞、後詞的全部條目。
    fn forget_word_in_triples(&mut self, word: &str) {
        self.triples.remove(word);
        self.triple_totals.remove(word);
        let mut emptied = Vec::new();
        for (earlier, by_previous) in &mut self.triples {
            by_previous.remove(word);
            let totals = self.triple_totals.entry(earlier.clone()).or_default();
            totals.remove(word);
            let mut cleared = Vec::new();
            for (previous, next) in by_previous.iter_mut() {
                if let Some(count) = next.remove(word) {
                    decrement(totals, previous, count);
                    if next.is_empty() {
                        cleared.push(previous.clone());
                    }
                }
            }
            for previous in cleared {
                by_previous.remove(&previous);
            }
            if by_previous.is_empty() {
                emptied.push(earlier.clone());
            }
        }
        for earlier in emptied {
            self.triples.remove(&earlier);
            self.triple_totals.remove(&earlier);
        }
    }

    /// 詞的出現次數。
    pub fn count(&self, word: &str) -> u32 {
        self.word_counts.get(word).copied().unwrap_or(0)
    }

    /// `word` 緊跟在 `previous` 之後的次數。
    pub fn pair(&self, previous: Option<&str>, word: &str) -> u32 {
        self.pairs
            .get(previous.unwrap_or(SENTENCE_START))
            .and_then(|next| next.get(word))
            .copied()
            .unwrap_or(0)
    }

    /// `word` 跟在 `earlier previous` 之後的次數（`earlier` 為 `None` 是句首標記）。
    pub fn triple(&self, earlier: Option<&str>, previous: &str, word: &str) -> u32 {
        self.triples
            .get(earlier.unwrap_or(SENTENCE_START))
            .and_then(|by_previous| by_previous.get(previous))
            .and_then(|next| next.get(word))
            .copied()
            .unwrap_or(0)
    }

    /// 不同的 (前詞, 後詞) 對數。
    pub fn pair_count(&self) -> usize {
        self.pairs.values().map(HashMap::len).sum()
    }

    /// 不同的 (前二詞, 前詞, 後詞) 條數。
    pub fn triple_count(&self) -> usize {
        self.triples
            .values()
            .flat_map(HashMap::values)
            .map(HashMap::len)
            .sum()
    }

    /// 二元與三元條目總數，決定什麼時候減半。
    pub fn transition_count(&self) -> usize {
        self.pair_count() + self.triple_count()
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// 把靜態模型給出的 `log P(word | previous)` 與個人機率插值後返回。
    ///
    /// 個人二元 P₂ = λ·c(v,w)/c(v) + (1-λ)·c(w)/N；這對上文 (u,v) 見過時再套一層絕對摺扣的三元：
    /// P₃ = max(c(u,v,w) − D, 0)/c(u,v) + D·N₁₊(u,v,·)/c(u,v)·P₂，沒見過的接續只拿回退的份額。
    /// 插值權重 μ = c(v)/(c(v)+K)，封頂 `max_confidence`：見過這個前詞越多越信個人資料，但永遠壓不死靜態模型。
    /// λ、K、封頂、三元折扣 D 都來自 `interpolation`（預設是本模組的常數）。前詞從沒見過時（句首用句首標記）原樣返回。
    pub fn blend(
        &self,
        context: Context<'_>,
        word: &str,
        base_log_prob: f64,
        interpolation: &Interpolation,
    ) -> f64 {
        let previous = context.previous.unwrap_or(SENTENCE_START);
        let Some(&context_total) = self.context_totals.get(previous) else {
            return base_log_prob;
        };
        if context_total == 0 || self.total == 0 {
            return base_log_prob;
        }
        let pair = f64::from(self.pair(context.previous, word));
        let unigram = f64::from(self.count(word)) / self.total as f64;
        let lambda = interpolation.lambda;
        let bigram = lambda * pair / f64::from(context_total) + (1.0 - lambda) * unigram;
        let personal = match context
            .previous
            .and_then(|p| self.triple_row(context.earlier, p))
        {
            Some((next, triple_total)) if triple_total > 0 => {
                let triple_total = f64::from(triple_total);
                let seen = next.get(word).copied().unwrap_or(0);
                let discount = interpolation.trigram_discount;
                let discounted = (f64::from(seen) - discount).max(0.0) / triple_total;
                let backoff = discount * next.len() as f64 / triple_total;
                discounted + backoff * bigram
            }
            _ => bigram,
        };
        let confidence = (f64::from(context_total)
            / (f64::from(context_total) + interpolation.confidence_k))
            .min(interpolation.max_confidence);
        let blended = (1.0 - confidence) * base_log_prob.exp() + confidence * personal;
        blended.max(f64::MIN_POSITIVE).ln()
    }

    /// 上文 (earlier, previous) 下的後詞表與總次數；沒見過這對上文返回 `None`。
    fn triple_row(
        &self,
        earlier: Option<&str>,
        previous: &str,
    ) -> Option<(&HashMap<String, u32>, u32)> {
        let earlier = earlier.unwrap_or(SENTENCE_START);
        let next = self.triples.get(earlier)?.get(previous)?;
        let total = self.triple_totals.get(earlier)?.get(previous).copied()?;
        Some((next, total))
    }

    /// 所有計數減半，去掉減到零的。
    fn decay(&mut self) {
        let mut pairs: HashMap<String, HashMap<String, u32>> = HashMap::new();
        for (previous, next) in &self.pairs {
            let halved = halve(next);
            if !halved.is_empty() {
                pairs.insert(previous.clone(), halved);
            }
        }
        let mut triples: HashMap<String, HashMap<String, HashMap<String, u32>>> = HashMap::new();
        for (earlier, by_previous) in &self.triples {
            for (previous, next) in by_previous {
                let halved = halve(next);
                if !halved.is_empty() {
                    triples
                        .entry(earlier.clone())
                        .or_default()
                        .insert(previous.clone(), halved);
                }
            }
        }
        *self = Self::from_counts(pairs, triples);
    }

    fn from_counts(
        pairs: HashMap<String, HashMap<String, u32>>,
        triples: HashMap<String, HashMap<String, HashMap<String, u32>>>,
    ) -> Self {
        let mut model = Self {
            pairs,
            triples,
            ..Self::default()
        };
        for (previous, next) in &model.pairs {
            for (word, count) in next {
                *model.context_totals.entry(previous.clone()).or_default() += count;
                *model.word_counts.entry(word.clone()).or_default() += count;
                model.total += u64::from(*count);
            }
        }
        for (earlier, by_previous) in &model.triples {
            let totals = model.triple_totals.entry(earlier.clone()).or_default();
            for (previous, next) in by_previous {
                *totals.entry(previous.clone()).or_default() += next.values().sum::<u32>();
            }
        }
        model
    }

    /// 從 TSV 解析；空行與 `#` 開頭的行跳過，三列是二元、四列是三元，格式不對的行返回行號。
    pub fn parse(source: &str) -> Result<Self, usize> {
        let (model, skipped) = Self::parse_lenient(source);
        match skipped.first() {
            Some(&line) => Err(line),
            None => Ok(model),
        }
    }

    /// 同 [`Self::parse`]，但壞行跳過而不是整份報錯，返回 (模型, 跳過的行號)。
    /// 學習資料檔案可能被崩潰寫壞一兩行，不能因此丟掉整個個人模型。
    pub fn parse_lenient(source: &str) -> (Self, Vec<usize>) {
        let mut pairs: HashMap<String, HashMap<String, u32>> = HashMap::new();
        let mut triples: HashMap<String, HashMap<String, HashMap<String, u32>>> = HashMap::new();
        let mut skipped = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let (context, word, count) = match fields.as_slice() {
                [previous, word, count] => (vec![*previous], *word, *count),
                [earlier, previous, word, count] => (vec![*earlier, *previous], *word, *count),
                _ => {
                    skipped.push(index + 1);
                    continue;
                }
            };
            let Ok(count) = count.trim().parse::<u32>() else {
                skipped.push(index + 1);
                continue;
            };
            if count == 0 || word.is_empty() {
                continue;
            }
            let slot = match context.as_slice() {
                [previous] => pairs.entry((*previous).to_owned()).or_default(),
                [earlier, previous] => triples
                    .entry((*earlier).to_owned())
                    .or_default()
                    .entry((*previous).to_owned())
                    .or_default(),
                _ => unreachable!(),
            };
            *slot.entry(word.to_owned()).or_default() += count;
        }
        (Self::from_counts(pairs, triples), skipped)
    }

    /// 序列化成 TSV 行（不含表頭），二元在前三元在後，各自排好序，便於人看和 diff。
    pub fn to_tsv(&self) -> String {
        let mut rows: Vec<(&str, &str, u32)> = self
            .pairs
            .iter()
            .flat_map(|(previous, next)| {
                next.iter()
                    .map(move |(word, count)| (previous.as_str(), word.as_str(), *count))
            })
            .collect();
        rows.sort_unstable();
        let mut out = String::new();
        for (previous, word, count) in rows {
            out.push_str(previous);
            out.push('\t');
            out.push_str(word);
            out.push('\t');
            out.push_str(&count.to_string());
            out.push('\n');
        }
        let mut rows: Vec<(&str, &str, &str, u32)> = self
            .triples
            .iter()
            .flat_map(|(earlier, by_previous)| {
                by_previous.iter().flat_map(move |(previous, next)| {
                    next.iter().map(move |(word, count)| {
                        (earlier.as_str(), previous.as_str(), word.as_str(), *count)
                    })
                })
            })
            .collect();
        rows.sort_unstable();
        for (earlier, previous, word, count) in rows {
            out.push_str(earlier);
            out.push('\t');
            out.push_str(previous);
            out.push('\t');
            out.push_str(word);
            out.push('\t');
            out.push_str(&count.to_string());
            out.push('\n');
        }
        out
    }
}

/// 計數表裡某一項減 `by`，減到零就刪掉。
fn decrement(table: &mut HashMap<String, u32>, key: &str, by: u32) {
    if let Some(count) = table.get_mut(key) {
        *count = count.saturating_sub(by);
        if *count == 0 {
            table.remove(key);
        }
    }
}

/// 從後詞表裡退回 `word` 最多 `times` 份，返回實際退回的份數；減到零就刪掉。
fn remove_times(next: &mut HashMap<String, u32>, word: &str, times: u32) -> u32 {
    let Some(count) = next.get_mut(word) else {
        return 0;
    };
    let removed = times.min(*count);
    *count -= removed;
    if *count == 0 {
        next.remove(word);
    }
    removed
}

/// 計數減半，去掉減到零的。
fn halve(next: &HashMap<String, u32>) -> HashMap<String, u32> {
    next.iter()
        .filter(|(_, count)| **count >= 2)
        .map(|(word, count)| (word.clone(), count / 2))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_transitions_and_round_trips_through_tsv() {
        let mut model = UserNgram::default();
        model.record(Context::START, "我");
        model.record(Context::after("我"), "想");
        model.record(Context::after_two("我", "想"), "去");
        model.record(Context::START, "我");
        model.record(Context::after("我"), "想");
        assert_eq!(model.count("我"), 2);
        assert_eq!(model.count("想"), 2);
        assert_eq!(model.pair(None, "我"), 2);
        assert_eq!(model.pair(Some("我"), "想"), 2);
        assert_eq!(model.pair(Some("我"), "去"), 0);
        assert_eq!(model.pair_count(), 3);
        // 句首詞不記三元；第二個詞的前二詞是句首標記
        assert_eq!(model.triple(None, "我", "想"), 2);
        assert_eq!(model.triple(Some("我"), "想", "去"), 1);
        assert_eq!(model.triple_count(), 2);

        let tsv = model.to_tsv();
        assert!(tsv.contains("<s>\t我\t2\n"));
        assert!(tsv.contains("<s>\t我\t想\t2\n"));
        assert!(tsv.contains("我\t想\t去\t1\n"));
        let restored = UserNgram::parse(&tsv).unwrap();
        assert_eq!(restored.pair(Some("我"), "想"), 2);
        assert_eq!(restored.count("去"), 1);
        assert_eq!(restored.pair_count(), 3);
        assert_eq!(restored.triple(None, "我", "想"), 2);
        assert_eq!(restored.triple_count(), 2);
        assert_eq!(restored.to_tsv(), tsv);
        assert_eq!(UserNgram::parse("我\t想\n").unwrap_err(), 1);
        assert_eq!(UserNgram::parse("<s>\t我\t想\t去\t1\n").unwrap_err(), 1);
        // 寬鬆解析：壞行跳過，好行照讀
        let (model, skipped) = UserNgram::parse_lenient("<s>\t我\t3\n坏行\n我\t想\tx\n我\t想\t2\n");
        assert_eq!(skipped, [2, 3]);
        assert_eq!(model.pair(None, "我"), 3);
        assert_eq!(model.pair(Some("我"), "想"), 2);
    }

    /// 舊版只有二元的檔案照讀，三元表為空。
    #[test]
    fn parses_bigram_only_files() {
        let model = UserNgram::parse("<s>\t我\t3\n我\t想\t2\n").unwrap();
        assert_eq!(model.pair(Some("我"), "想"), 2);
        assert_eq!(model.triple_count(), 0);
        assert_eq!(
            model.blend(
                Context::after_two("我", "想"),
                "去",
                -5.0,
                &Interpolation::DEFAULT
            ),
            model.blend(Context::after("想"), "去", -5.0, &Interpolation::DEFAULT)
        );
    }

    #[test]
    fn blend_favors_seen_continuations_but_never_kills_unseen_ones() {
        let mut model = UserNgram::default();
        let base_ba = (-9.0_f64).exp().ln();
        // 前詞沒見過：原樣返回
        assert_eq!(
            model.blend(
                Context::after("吃饭"),
                "把",
                base_ba,
                &Interpolation::DEFAULT
            ),
            base_ba
        );
        // 選過兩次 吃飯 → 把：個人證據抬上來
        model.record(Context::after("吃饭"), "把");
        model.record(Context::after("吃饭"), "把");
        let lifted = model.blend(
            Context::after("吃饭"),
            "把",
            base_ba,
            &Interpolation::DEFAULT,
        );
        assert!(lifted > -2.5, "{lifted}");
        // 沒跟在 吃飯 後面出現過的 吧 只是打折，不會被壓死
        let base_ba_particle = -2.0;
        let discounted = model.blend(
            Context::after("吃饭"),
            "吧",
            base_ba_particle,
            &Interpolation::DEFAULT,
        );
        assert!(discounted < base_ba_particle);
        assert!(
            discounted
                > base_ba_particle + (1.0 - Interpolation::DEFAULT.max_confidence).ln() - 1e-9
        );
    }

    /// 三元分辨二元分不開的接續：「想 → 去」在「我想」後面和「不想」後面偏好不同。
    #[test]
    fn trigram_separates_continuations_the_bigram_mixes() {
        let mut model = UserNgram::default();
        for _ in 0..3 {
            model.record(Context::after_two("我", "想"), "去");
            model.record(Context::after_two("不", "想"), "要");
        }
        let base = -6.0;
        let after_wo = model.blend(
            Context::after_two("我", "想"),
            "去",
            base,
            &Interpolation::DEFAULT,
        );
        let after_bu = model.blend(
            Context::after_two("不", "想"),
            "去",
            base,
            &Interpolation::DEFAULT,
        );
        let bigram_only = model.blend(Context::after("想"), "去", base, &Interpolation::DEFAULT);
        assert!(after_wo > bigram_only, "{after_wo} vs {bigram_only}");
        assert!(bigram_only > after_bu, "{bigram_only} vs {after_bu}");
        // 三元上文沒見過的（「很想」）退回二元
        assert_eq!(
            model.blend(
                Context::after_two("很", "想"),
                "去",
                base,
                &Interpolation::DEFAULT
            ),
            bigram_only
        );
        // 沒見過的接續在見過的三元上文裡仍拿到回退份額，不會被壓死
        let unseen = model.blend(
            Context::after_two("我", "想"),
            "要",
            base,
            &Interpolation::DEFAULT,
        );
        assert!(unseen > base + (1.0 - Interpolation::DEFAULT.max_confidence).ln() - 1e-9);
        assert!(unseen < after_wo);
    }

    #[test]
    fn forget_word_removes_it_from_every_position() {
        let mut model = UserNgram::default();
        model.record(Context::START, "我");
        model.record(Context::after("我"), "想");
        model.record(Context::after_two("我", "想"), "去");
        model.record(Context::after("我"), "去");
        assert_eq!(model.forget_word("想"), 2);
        assert_eq!(model.count("想"), 0);
        assert_eq!(model.pair(Some("我"), "想"), 0);
        assert_eq!(model.pair(Some("想"), "去"), 0);
        assert_eq!(model.pair(Some("我"), "去"), 1);
        assert_eq!(model.count("去"), 1);
        assert_eq!(model.pair_count(), 2);
        assert_eq!(model.triple(None, "我", "想"), 0);
        assert_eq!(model.triple(Some("我"), "想", "去"), 0);
        assert_eq!(model.triple(None, "我", "去"), 1);
        assert_eq!(model.triple_count(), 1);
        assert_eq!(model.forget_word("没有"), 0);
        // 作為前二詞刪掉
        assert_eq!(model.forget_word("我"), 2);
        assert_eq!(model.triple_count(), 0);
        assert!(model.is_empty());
    }

    #[test]
    fn decays_when_too_many_transitions() {
        let mut model = UserNgram::default();
        // 每條記錄一個二元加一個三元
        for i in 0..MAX_USER_TRANSITIONS / 2 {
            model.record(Context::after("甲"), &format!("乙{i}"));
        }
        for _ in 0..3 {
            model.record(Context::after("甲"), "丙");
        }
        // 超上限後減半：單次的都被忘掉，選過三次的還留著
        model.record(Context::after("甲"), "丁");
        assert!(model.transition_count() <= 4);
        assert!(model.pair(Some("甲"), "丙") >= 1);
        assert!(model.triple(None, "甲", "丙") >= 1);
        assert_eq!(model.pair(Some("甲"), "乙0"), 0);
        assert_eq!(model.triple(None, "甲", "乙0"), 0);
    }

    #[test]
    fn unrecord_reverses_record_and_ignores_unknown_transitions() {
        let mut model = UserNgram::default();
        model.record_times(Context::START, "好的", 2);
        model.record(Context::after("好的"), "啊");
        model.unrecord(Context::START, "好的", 2);
        assert_eq!(model.pair(None, "好的"), 0);
        assert_eq!(model.count("好的"), 0);
        assert_eq!(model.pair(Some("好的"), "啊"), 1);
        assert_eq!(model.triple(None, "好的", "啊"), 1);
        assert_eq!(model.pair_count(), 1);
        model.unrecord(Context::after("没有"), "的", 5);
        model.unrecord(Context::after("好的"), "啊", 5);
        assert!(model.is_empty());
        assert_eq!(model.transition_count(), 0);
    }
}
