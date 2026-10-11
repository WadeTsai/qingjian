//! 拼音側的候選生成：切分、拼寫糾錯、詞級查詢與排序，再補上整句、英文、快捷與 emoji 候選。

use super::*;

impl Engine {
    /// 拼音側（全拼 / 雙拼 / 注音）的候選生成：整段作用域是一串讀音。
    pub(super) fn query_phonetic(
        &self,
        keys: &str,
        rest: String,
        start: Instant,
    ) -> Result<Query, ParseError> {
        // 雙拼先解成全拼（音節間已用 `'` 連好，切分沒有歧義），之後與全拼同路；解不動的鍵當尾巴
        let decoded = self.decode(keys);
        let scope: &str = decoded.as_ref().map_or(keys, |d| d.pinyin());
        // 末尾是英文詞（`woxiangxuehaorust`）：拼音候選與整句只按頭段算，尾段整個跟在整句後面。
        // 整段也能讀成拼音時（`database`、`…rust` 當簡拼）兩種讀法比分，英文贏了才按頭段算，
        // 輸了整段按拼音讀、英文讀法排在拼音整句後面
        let english_tail = if decoded.is_none() {
            self.split_english_tail(keys)
        } else {
            None
        };
        let head_wins = english_tail
            .as_ref()
            .is_some_and(|t| !t.competes || self.mixed_beats_plain(keys, t));
        let parsed = match (&decoded, &english_tail) {
            (Some(d), _) => d
                .segmentation()
                .map(|s| (vec![s], d.tail()))
                .ok_or(ParseError::NoSegmentation),
            (None, Some(tail)) if head_wins => {
                parser::segment(&keys[..tail.head_len]).map(|s| (s, ""))
            }
            _ => segment_longest_prefix(keys),
        };
        // 連第一個字母都切不動（`impor`）：拼音這邊沒戲，但英文詞 / 補全、快捷候選還可以有
        let (segmentations, tail) = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                let mut items = Vec::new();
                self.insert_english(&mut items, true);
                self.insert_shortcuts(&mut items, keys);
                if items.is_empty() {
                    return Err(error);
                }
                return Ok(Query {
                    segmentations: Vec::new(),
                    candidates: CandidateList { items },
                    tail: keys.to_owned(),
                    text: self.composition.text().to_owned(),
                    cursor: self.composition.cursor(),
                    rest,
                    decoded_keys: self.shuangpin.is_some() || self.zhuyin,
                    shuangpin_raw_preedit: self.shuangpin.is_some() && self.shuangpin_raw_preedit,
                    typed_display: decoded.as_ref().map(|d| d.marked()),
                    correction: None,
                    aux: None,
                    timings: Timings {
                        parse: start.elapsed(),
                        lookup: Duration::ZERO,
                        rank: Duration::ZERO,
                    },
                });
            }
        };
        // 拼音「不像話」時試拼寫糾錯；糾正生效則按糾正後的切分查詞，原串只用來記學習與顯示
        let unlikely = correction::unlikely_pinyin(segmentations.first(), tail)
            || correction::trailing_single_letter(segmentations.first());
        let correction = if unlikely {
            self.active_correction(scope)
        } else {
            None
        };
        let (segmentations, tail): (Vec<Segmentation>, &str) = match &correction {
            Some(c) => (vec![c.segmentation.clone()], ""),
            None => (segmentations, tail),
        };
        let parse = start.elapsed();

        let start = Instant::now();
        let mut scored = Vec::new();
        // 不同切分共享很多字首（`zh g d o…` 的各種切法前幾段一樣），同一次查詢裡同一個模式只查一遍
        let mut memo: HashMap<String, Vec<Match<'_>>> = HashMap::new();
        for segmentation in &segmentations {
            let mut patterns = segmentation.patterns();
            let count = patterns.len();
            let last = &segmentation.syllables[count - 1];
            // 最後一個音節即使打完了也可能還沒打完（`xia` 可能是 `xiang` 的字首），按字首查；雙拼兩鍵就是定局
            if last.complete && decoded.is_none() && parser::is_syllable_prefix(&last.text) {
                patterns[count - 1].complete = false;
            }
            // 詞級候選只按敲的原樣與模糊音查，敲錯變體只進整句詞圖（它的候選從那邊插進來）：
            // 詞級排序把音節數對得上的排最前，敲錯命中的詞（`kaif` → 咖啡）會把更長的原樣詞擠到後面
            let expanded = self.fuzzy.expand(&patterns);
            let positions = expanded.positions();
            let abbreviated = abbreviated_count(&patterns);
            // 沒有替代寫法時每條命中都是敲的原音節，`penalty` 直接給 0（單字母簡拼能命中幾萬條）
            let hits = self.lookup_all(&positions);
            scored.reserve(hits.len());
            for hit in hits {
                let full_last = last.complete
                    && hit.syllables().nth(count - 1) == Some(patterns[count - 1].text);
                scored.push(Scored {
                    hit,
                    full_last,
                    coverage: segmentation.letters(),
                    abbreviated,
                    weight: self.learner.weight(hit.text),
                    penalty: expanded.penalty(hit.syllables()),
                });
            }
            // 輸入的字首也出候選（`kaifazhe` → 開發、開），否則長句沒法逐詞上屏。
            // 只收音節數正好等於字首長度的詞，更長的詞會與輸入後面的音節衝突。
            // 字首不含最後一個位置，因此可複用上面的擴充套件結果。
            for prefix_len in (1..count).rev() {
                let prefix = &patterns[..prefix_len];
                let prefix_letters: usize = prefix.iter().map(|p| p.text.len()).sum();
                let hits = memo
                    .entry(pattern_key(prefix))
                    .or_insert_with(|| self.lookup_exact_all(&positions[..prefix_len]));
                let abbreviated = abbreviated_count(prefix);
                for hit in hits.iter().copied() {
                    scored.push(Scored {
                        // 對整個輸入來說它不是精確命中，只是覆蓋了前面一部分
                        hit: Match {
                            exact: false,
                            ..hit
                        },
                        full_last: true,
                        coverage: prefix_letters,
                        abbreviated,
                        weight: self.learner.weight(hit.text),
                        penalty: expanded.penalty(hit.syllables()),
                    });
                }
            }
        }
        let lookup = start.elapsed();

        let start = Instant::now();
        // 再往後翻也翻不到的候選不必再造：單字母簡拼能命中兩萬個詞，排完序只留前面這些。
        // 同輸入串（候選覆蓋的那段字母）下選過的優先；上下文是上一個上屏的詞（句首為 None）：
        // `ba` 在「做了」後面出 吧、句首出 把
        let log_total = (self.total_frequency() as f64).max(1.0).ln();
        let letters = choice_key(scope, scope.len());
        ranking::rank(&mut scored, MAX_CANDIDATES, |item| {
            let hit = &item.hit;
            // 糾錯生效時覆蓋的是糾正後的字母，換算回原串再查「這個輸入串下選過什麼」
            let covered = correction
                .as_ref()
                .map_or(item.coverage, |c| c.edit.to_original(item.coverage));
            let choice = letters
                .get(..covered)
                .map_or(0, |input| self.learner.choice_weight(input, hit.text));
            let log_prob = sentence::transition_log_prob(
                &*self.language_model,
                self.personal(),
                self.chain.context(),
                hit.text,
                sentence::fallback_log_prob(hit.frequency, log_total),
            );
            (choice, log_prob)
        });
        // 輔碼態：詞庫候選按碼段**反向**過濾（逐個問「有沒有以碼段開頭的碼」），無碼詞直接隱藏；
        // 命中的按「完全匹配碼 > 碼長降序 > 原詞頻序」重排（stable sort 保住 rank 排好的原序）。
        // 碼段為空（剛敲下觸發鍵）時不過濾，候選與純拼音態一模一樣。
        let aux_code = self.aux_filter();
        let mut items: Vec<Candidate> = Vec::with_capacity(scored.len());
        match aux_code {
            // 沒在篩碼：首條碼只在顯示開關開著或已在輔碼態時掛，否則不逐候選查碼
            None => items.extend(scored.into_iter().map(|item| {
                let first = if self.aux_show || self.aux_code.is_some() {
                    self.matching_code(item.hit.text, "")
                } else {
                    None
                };
                chinese_candidate(&item, first)
            })),
            Some(code) => {
                let mut kept: Vec<(usize, &str)> = scored
                    .iter()
                    .enumerate()
                    .filter_map(|(index, item)| {
                        self.matching_code(item.hit.text, code)
                            .map(|hit| (index, hit))
                    })
                    .collect();
                kept.sort_by(|a, b| {
                    (b.1.len() == code.len())
                        .cmp(&(a.1.len() == code.len()))
                        .then_with(|| b.1.len().cmp(&a.1.len()))
                });
                items.extend(
                    kept.into_iter()
                        .map(|(index, hit)| chinese_candidate(&scored[index], Some(hit))),
                );
            }
        }
        // 附加候選（英文尾段與補全、快捷、整句、emoji）都沒有碼：輔碼篩詞時一律不出
        if aux_code.is_none() {
            // 中文優先：整句先進去佔第一，英文詞緊跟其後（第二）；關掉時英文詞先進、整句排在開頭的英文後面
            if self.chinese_first {
                self.insert_sentence(
                    &mut items,
                    &segmentations,
                    correction.is_none(),
                    english_tail.as_ref().filter(|_| correction.is_none()),
                    head_wins,
                );
                self.insert_english(&mut items, unlikely);
            } else {
                self.insert_english(&mut items, unlikely);
                self.insert_sentence(
                    &mut items,
                    &segmentations,
                    correction.is_none(),
                    english_tail.as_ref().filter(|_| correction.is_none()),
                    head_wins,
                );
            }
            // 快捷候選按敲的鍵認（`rq` 日期），雙拼下也是
            self.insert_shortcuts(&mut items, keys);
            self.insert_emoji(&mut items);
        }
        let rank = start.elapsed();

        // 按頭段算時英文尾段不參與拼音候選，顯示上跟在切分後面：`wo'xiang'xue'hao'rust`
        let tail = english_tail
            .as_ref()
            .filter(|_| head_wins)
            .map_or(tail, |t| &keys[t.head_len..]);
        let typed_display = decoded.as_ref().map(|d| d.marked()).or_else(|| {
            // 中文模式下 Shift 敲的大寫：匹配按小寫算，拼音行仍按敲的樣子顯示（`Cpan`）
            (correction.is_none() && self.composition.has_shifted())
                .then(|| join_marked_typed(&self.composition.typed_scope(), &segmentations, tail))
        });
        Ok(Query {
            segmentations,
            candidates: CandidateList { items },
            tail: tail.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: self.shuangpin.is_some() || self.zhuyin,
            shuangpin_raw_preedit: self.shuangpin.is_some() && self.shuangpin_raw_preedit,
            typed_display,
            correction,
            aux: self.aux_segment(),
            timings: Timings {
                parse,
                lookup,
                rank,
            },
        })
    }
}
