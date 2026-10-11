//! 形碼查詢：編碼按字首查碼錶，沒有切分、沒有整句；以及與拼音的混輸。

use std::collections::HashSet;

use super::*;

impl Engine {
    /// 形碼方案（五筆）的候選：編碼打全的詞排在同字首的更長編碼詞前面，其餘按詞頻與上下文。
    ///
    /// 拼音那一套在這裡都不成立：編碼不需要切分，沒有簡拼與模糊音，一處編輯的拼寫糾錯更不適用，
    /// 整句轉換、中英混輸與神經重排也沒有可以展開的東西。反過來，譯詞標註、生詞記錄、輸入日誌、
    /// 使用者選擇學習與個人 n-gram 都按上屏的詞工作，與拼音方案共用同一條路。
    pub(super) fn query_code(&self, keys: &str, rest: String, start: Instant) -> Query {
        self.query_code_counted(keys, rest, start).0
    }

    /// 同 [`Self::query_code`]，另帶回排在最前面的「編碼打全」的候選有幾條（混輸按它分兩段）。
    fn query_code_counted(&self, keys: &str, rest: String, start: Instant) -> (Query, usize) {
        let table = self.code.as_ref().expect("只在形码方案下调用");
        // 編碼以外的字元（`no-way` 的 `-`）不是編碼，交給原樣上屏那條路
        if !keys.chars().all(|c| c.is_ascii_lowercase()) {
            return (self.query_raw(keys, rest, start), 0);
        }
        let parse = start.elapsed();

        let start = Instant::now();
        let log_total = (table.total_frequency() as f64).max(1.0).ln();
        let mut scored: Vec<Scored<'_>> = table
            .lookup(keys, MAX_CANDIDATES)
            .into_iter()
            .map(|hit| Scored {
                hit,
                // 編碼是字首匹配：沒有「最後一個音節打完了」這回事，也不吃簡拼、模糊音與敲錯
                full_last: true,
                coverage: keys.len(),
                abbreviated: 0,
                weight: self.learner.weight(hit.text),
                penalty: 0.0,
            })
            .collect();
        let lookup = start.elapsed();

        // 同一段編碼下選過的詞優先，再按上下文得分（上一個上屏的詞）；詞頻只在模型不認識時兜底
        let start = Instant::now();
        let letters = choice_key(keys, keys.len());
        ranking::rank(&mut scored, MAX_CANDIDATES, |item| {
            let choice = letters
                .get(..item.coverage)
                .map_or(0, |input| self.learner.choice_weight(input, item.hit.text));
            let log_prob = sentence::transition_log_prob(
                &*self.language_model,
                self.personal(),
                self.chain.context(),
                item.hit.text,
                sentence::fallback_log_prob(item.hit.frequency, log_total),
            );
            (choice, log_prob)
        });
        let rank = start.elapsed();
        // `exact` 是排序的最高位，打全的都在最前面
        let exact = scored.iter().take_while(|s| s.hit.exact).count();
        let items: Vec<Candidate> = scored
            .into_iter()
            .map(|s| Candidate {
                text: s.hit.text.to_owned(),
                kind: CandidateKind::Code,
                // 編碼不是拼音音節：候選窗按音節高亮的部分對形碼沒有意義，留空
                syllables: Vec::new(),
                reading: None,
                translation: None,
                aux_code: None,
            })
            .collect();
        let query = Query {
            // 形碼沒有切分：preedit 的顯示串靠 `tail` 原樣帶出去（見 `Query::marked_text`）
            segmentations: Vec::new(),
            candidates: CandidateList { items },
            tail: keys.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: false,
            shuangpin_raw_preedit: false,
            typed_display: None,
            correction: None,
            aux: None,
            timings: Timings {
                parse,
                lookup,
                rank,
            },
        };
        (query, exact)
    }

    /// 混輸：形碼與拼音兩邊都出候選。**編碼打全的形碼詞在最前，其次拼音，只命中字首的形碼詞墊後。**
    ///
    /// 打全的編碼是精確的（`ga` 就是 開）；只敲了字首的形碼詞一律放前面的話，`kai` 的首選會變成
    /// 編碼 `kaik` 的 中共黨員，拼音就沒法用了。拼音那條路給不出解析時（`ggll` 切不成音節）不算失敗，
    /// 整個查詢按形碼的結果走；五筆碼最長 4 位，第 5 個字母起形碼查不到東西，自然只剩拼音。
    pub(super) fn query_mixed(
        &self,
        keys: &str,
        rest: String,
        start: Instant,
    ) -> Result<Query, ParseError> {
        let (code, exact) = self.query_code_counted(keys, rest.clone(), start);
        let mut query = match self.query_phonetic(keys, rest, start) {
            Ok(query) => query,
            Err(error) => {
                // 拼音讀不出來，但形碼有東西：把形碼那條留著，出錯只在兩邊都空時才算
                if code.candidates.items.is_empty() {
                    return Err(error);
                }
                return Ok(code);
            }
        };
        let mut prefixed = code.candidates.items;
        let exact = exact.min(prefixed.len());
        let tail = prefixed.split_off(exact);
        // 同一個詞可能兩邊都命中，按文本去重，靠前的那條留著
        let mut seen: HashSet<String> = HashSet::new();
        let combined: Vec<Candidate> = prefixed
            .into_iter()
            .chain(std::mem::take(&mut query.candidates.items))
            .chain(tail)
            .filter(|candidate| seen.insert(candidate.text.clone()))
            .take(MAX_CANDIDATES)
            .collect();
        query.candidates.items = combined;
        Ok(query)
    }
}
