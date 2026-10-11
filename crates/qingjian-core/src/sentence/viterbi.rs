//! 詞圖上的最優路徑：bigram Viterbi + 束搜尋。
//!
//! 狀態只按前一個詞分（束寬內），個人三元要的前二詞取前驅節點的回指（它那條最優路徑上的前一個詞）：
//! 不擴狀態，代價是三元上下文是近似的，個人資料量下夠用。

use qingjian_dictionary::{Dictionary, Match, SyllablePattern};

use super::{
    ABBREVIATED_SPAN_CANDIDATES, BEAM_WIDTH, Context, Conversion, LanguageModel,
    MAX_WORD_SYLLABLES, MIN_PARTIAL_LETTERS, Personal, SPAN_CANDIDATES, SentenceWord, SpanCache,
    SpanWord, fallback_log_prob, transition_log_prob,
};
use crate::ranking::weight_bonus;

/// 詞庫裡沒有的孤立音節（罕見音節沒有單字）按這個 log 機率兜底，讓路徑總能走通。
const UNKNOWN_LOG_PROB: f64 = -30.0;

/// 一條部分路徑的末尾節點。
struct Node {
    /// 這個詞從第幾個音節開始。
    start: usize,

    /// 詞。
    text: String,

    /// 詞的音節。
    syllables: Vec<String>,

    /// 到此為止的累計得分。
    score: f64,

    /// 累計得分裡靜態模型的部分（見 `Conversion::static_score`）。
    static_score: f64,

    /// 前驅在 `nodes[start]` 裡的下標；`start == 0` 時無意義。
    back: usize,

    /// 是佔位音節。
    placeholder: bool,

    /// 到此為止路徑上模糊音 / 敲錯變體的代價之和（已從 `score` 里扣掉，另記一份給呼叫方判斷路徑是不是原樣）。
    penalty: f64,
}

/// 把音節序列轉成最可能的詞序列。`positions` 每個位置是若干寫法（第一種是敲的，其餘是模糊音 / 敲錯變體），
/// `cost(位置, 命中的音節)` 是那個位置命中這種寫法要扣的分（敲的原樣 0），
/// `weight` 是使用者選擇次數，`personal` 是個人 n-gram 與插值引數（沒有個人資料就傳 [`Personal::NONE`]），`cache` 是格子候選的快取
/// （呼叫方保證它與詞庫、`weight`、`personal`、`cost` 一致，這些一變就清）。
///
/// 簡拼位置（`w x q`）按字首取詞：每個格子的候選會多得多，由語言模型在路徑上分辨。
/// 全拼句子末尾的字首太短時不算它（多半是沒打完的音節）；前面已有簡拼的句子裡末尾單字母就是一個音節。
pub fn convert(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Option<Conversion> {
    convert_with(
        dictionaries,
        positions,
        false,
        model,
        personal,
        weight,
        cost,
        cache,
    )
}

/// 同 [`convert`]，但全拼句子末尾的單字母也當一個音節讀（`huo z…` → 或者）：
/// 給「整段拼音讀法」與別的讀法比分用，比分要兩邊覆蓋同樣多的字母。
pub fn convert_whole(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Option<Conversion> {
    convert_with(
        dictionaries,
        positions,
        true,
        model,
        personal,
        weight,
        cost,
        cache,
    )
}

/// [`convert`] 與 [`convert_whole`] 的共同實現，`keep_partial` 選哪種；只要最優的一條。
#[allow(clippy::too_many_arguments)]
pub fn convert_with(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    keep_partial: bool,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Option<Conversion> {
    convert_paths(
        dictionaries,
        positions,
        keep_partial,
        1,
        model,
        personal,
        weight,
        cost,
        cache,
    )
    .into_iter()
    .next()
}

/// 得分最高的前 `k` 條路徑（最多束寬條，按得分降序，文本相同的只留一條）：給重打分用。
#[allow(clippy::too_many_arguments)]
pub fn convert_paths(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    keep_partial: bool,
    k: usize,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Vec<Conversion> {
    let Some((last, head)) = positions.split_last() else {
        return Vec::new();
    };
    let Some(&last) = last.first() else {
        return Vec::new();
    };
    let abbreviated_head = head.iter().any(|p| p.first().is_none_or(|t| !t.complete));
    let positions = if keep_partial
        || last.complete
        || abbreviated_head
        || last.text.len() >= MIN_PARTIAL_LETTERS
    {
        positions
    } else {
        head
    };
    let n = positions.len();
    if n == 0 || k == 0 {
        return Vec::new();
    }
    let total: f64 = dictionaries
        .iter()
        .map(|d| d.total_frequency() as f64)
        .sum::<f64>()
        .max(1.0);
    let log_total = total.ln();

    // nodes[i]：覆蓋前 i 個音節、以某個詞結尾的部分路徑；nodes[0] 是虛擬起點
    let mut nodes: Vec<Vec<Node>> = (0..=n).map(|_| Vec::new()).collect();
    nodes[0].push(Node {
        start: 0,
        text: String::new(),
        syllables: Vec::new(),
        score: 0.0,
        static_score: 0.0,
        back: 0,
        placeholder: false,
        penalty: 0.0,
    });
    for start in 0..n {
        prune(&mut nodes[start]);
        if nodes[start].is_empty() {
            continue;
        }
        let mut any = false;
        for end in start + 1..=n.min(start + MAX_WORD_SYLLABLES) {
            let span = &positions[start..end];
            let hits = cache.get_or_insert_with(SpanCache::key(span), || {
                span_candidates(dictionaries, span, start, personal, &weight, &cost)
            });
            if hits.is_empty() {
                continue;
            }
            any = true;
            for hit in hits.iter() {
                let bonus = weight_bonus(weight(&hit.text));
                let fallback = fallback_log_prob(hit.frequency, log_total);
                let (score, back) =
                    best_predecessor(&nodes, start, &hit.text, model, personal, fallback);
                let previous = &nodes[start][back];
                let penalty = previous.penalty + hit.penalty;
                let static_step = model
                    .log_prob((start > 0).then_some(previous.text.as_str()), &hit.text)
                    .unwrap_or(fallback);
                let static_score = previous.static_score + static_step;
                nodes[end].push(Node {
                    start,
                    text: hit.text.clone(),
                    syllables: hit.syllables.clone(),
                    score: score + bonus - hit.penalty,
                    static_score,
                    back,
                    placeholder: false,
                    penalty,
                });
            }
        }
        // 這個音節連單字都查不到：用音節本身佔位，別讓整句斷掉
        if !any {
            let text = positions[start][0].text;
            let (score, back) = best_predecessor(
                &nodes,
                start,
                text,
                &NoModel,
                Personal::NONE,
                UNKNOWN_LOG_PROB,
            );
            let penalty = nodes[start][back].penalty;
            let static_score = nodes[start][back].static_score + UNKNOWN_LOG_PROB;
            nodes[start + 1].push(Node {
                start,
                text: text.to_owned(),
                syllables: vec![text.to_owned()],
                score,
                static_score,
                back,
                placeholder: true,
                penalty,
            });
        }
    }
    prune(&mut nodes[n]);
    let mut paths: Vec<Conversion> = Vec::with_capacity(k.min(nodes[n].len()));
    for index in 0..nodes[n].len() {
        if paths.len() >= k {
            break;
        }
        let conversion = backtrack(&nodes, n, index);
        if !paths.iter().any(|p| p.text == conversion.text) {
            paths.push(conversion);
        }
    }
    paths
}

/// 從 `nodes[position][index]` 回溯出整條路徑。
fn backtrack(nodes: &[Vec<Node>], mut position: usize, mut index: usize) -> Conversion {
    let score = nodes[position][index].score;
    let static_score = nodes[position][index].static_score;
    let penalty = nodes[position][index].penalty;
    let mut words: Vec<SentenceWord> = Vec::new();
    while position > 0 {
        let node = &nodes[position][index];
        words.push(SentenceWord {
            text: node.text.clone(),
            syllables: node.syllables.clone(),
            placeholder: node.placeholder,
        });
        position = node.start;
        index = node.back;
    }
    words.reverse();
    let mut text = String::new();
    let mut syllables = Vec::new();
    for word in &words {
        text.push_str(&word.text);
        syllables.extend(word.syllables.iter().cloned());
    }
    Conversion {
        text,
        syllables,
        words,
        score,
        static_score,
        penalty,
    }
}

/// 佔位音節不問語言模型。
struct NoModel;

impl LanguageModel for NoModel {
    fn log_prob(&self, _previous: Option<&str>, _word: &str) -> Option<f64> {
        None
    }
}

/// 一個格子裡的候選詞：所有詞庫的精確命中，按詞頻（加使用者選擇次數與個人出現次數，替代寫法命中的按代價打折）取前幾個。
/// 個人次數只在這裡保證使用者常用的同音詞進得了格子，不進路徑打分（那是 n-gram 的事）；
/// 打折讓敲錯變體命中的詞只在原樣命中不夠多時才進格子，而常用詞（關係）即使打折也留得住。
/// 格子裡有簡拼位置時命中的是一大片不同讀音的詞，多留一些讓語言模型去挑。
fn span_candidates(
    dictionaries: &[&Dictionary],
    span: &[Vec<SyllablePattern<'_>>],
    start: usize,
    personal: Personal<'_>,
    weight: &impl Fn(&str) -> u32,
    cost: &impl Fn(usize, &str) -> f64,
) -> Vec<SpanWord> {
    let alternatives = span.iter().any(|p| p.len() > 1);
    let penalty_of = |m: &Match<'_>| {
        if !alternatives {
            return 0.0;
        }
        m.syllables()
            .enumerate()
            .map(|(index, syllable)| cost(start + index, syllable))
            .sum::<f64>()
    };
    // 得分先算好再排：單字母簡拼的格子能命中幾千條，比較器裡每次查兩張表會讓排序佔掉十幾毫秒
    let mut scored: Vec<(f64, f64, Match<'_>)> = dictionaries
        .iter()
        .flat_map(|d| d.lookup_exact_alt(span))
        .map(|m| {
            let seen = weight(m.text) + personal.count(m.text);
            let penalty = penalty_of(&m);
            let score = f64::from(m.frequency) * (1.0 + f64::from(seen)) * (-penalty).exp();
            (score, penalty, m)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.dedup_by(|a, b| a.2.text == b.2.text);
    let abbreviated = span.iter().any(|p| p.iter().any(|t| !t.complete));
    scored.truncate(if abbreviated {
        ABBREVIATED_SPAN_CANDIDATES
    } else {
        SPAN_CANDIDATES
    });
    scored
        .into_iter()
        .map(|(_, penalty, hit)| SpanWord {
            text: hit.text.to_owned(),
            syllables: hit.syllables().map(str::to_owned).collect(),
            frequency: hit.frequency,
            penalty,
        })
        .collect()
}

/// 在 `nodes[start]` 的前驅裡挑讓 `word` 得分最高的那條，返回 (累計得分, 前驅下標)。
/// 轉移機率先問靜態模型（不認識就用詞庫兜底值），再與個人 n-gram 插值；前二詞是前驅自己的前驅（回指）。
fn best_predecessor(
    nodes: &[Vec<Node>],
    start: usize,
    word: &str,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    fallback: f64,
) -> (f64, usize) {
    let mut best = (f64::NEG_INFINITY, 0);
    for (index, previous) in nodes[start].iter().enumerate() {
        let context = if start == 0 {
            Context::START
        } else {
            Context {
                previous: Some(previous.text.as_str()),
                earlier: (previous.start > 0)
                    .then(|| nodes[previous.start][previous.back].text.as_str()),
            }
        };
        let score = previous.score + transition_log_prob(model, personal, context, word, fallback);
        if score > best.0 {
            best = (score, index);
        }
    }
    best
}

/// 按得分降序只留束寬條。
fn prune(nodes: &mut Vec<Node>) {
    nodes.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    nodes.truncate(BEAM_WIDTH);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sentence::{NoLanguageModel, UserNgram};

    const SAMPLE: &str = "我\two\t900000\n想\txiang\t500000\n去\tqu\t400000\n吃\tchi\t300000\n饭\tfan\t200000\n\
        吃饭\tchi fan\t100000\n我想\two xiang\t600000\n翔\txiang\t3000\n区\tqu\t100000\n卧\two\t2000\n\
        开发\tkai fa\t9000\n开\tkai\t20000\n发\tfa\t30000\n开放\tkai fang\t20000\n";

    fn complete<'a>(syllables: &[&'a str]) -> Vec<Vec<SyllablePattern<'a>>> {
        syllables
            .iter()
            .map(|s| vec![SyllablePattern::complete(s)])
            .collect()
    }

    fn unigram(
        dictionary: &Dictionary,
        patterns: &[Vec<SyllablePattern<'_>>],
    ) -> Option<Conversion> {
        convert(
            &[dictionary],
            patterns,
            &NoLanguageModel,
            Personal::NONE,
            |_| 0,
            |_, _| 0.0,
            &mut SpanCache::default(),
        )
    }

    #[test]
    fn picks_common_words_over_characters() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "xiang", "qu", "chi", "fan"]);
        let conversion = unigram(&dictionary, &patterns).unwrap();
        assert_eq!(conversion.text, "我想去吃饭");
        assert_eq!(conversion.word_count(), 3); // 我想 / 去 / 吃飯
        assert_eq!(conversion.words[0].text, "我想");
        assert_eq!(conversion.words[0].syllables, ["wo", "xiang"]);
        assert_eq!(conversion.syllables.len(), 5);
    }

    #[test]
    fn user_weight_lifts_near_ties_but_is_capped() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        // wo qu 沒有整詞：臥 / 我 詞頻差 450 倍（log 差 6.1），加分封頂後選多少次都翻不過來
        let patterns = complete(&["wo", "qu"]);
        let lifted = |count: u32| {
            convert(
                &[&dictionary],
                &patterns,
                &NoLanguageModel,
                Personal::NONE,
                |t| if t == "卧" { count } else { 0 },
                |_, _| 0.0,
                &mut SpanCache::default(),
            )
            .unwrap()
            .text
        };
        assert_eq!(lifted(20), "我去");
        assert_eq!(lifted(500), "我去");
        // 去 / 區 差 4 倍（log 差 1.4）：選過十幾次就翻過來
        let patterns = complete(&["wo", "qu"]);
        let lifted = |count: u32| {
            convert(
                &[&dictionary],
                &patterns,
                &NoLanguageModel,
                Personal::NONE,
                |t| if t == "区" { count } else { 0 },
                |_, _| 0.0,
                &mut SpanCache::default(),
            )
            .unwrap()
            .text
        };
        assert_eq!(lifted(2), "我去");
        assert_eq!(lifted(20), "我区");
    }

    #[test]
    fn partial_last_syllable_of_a_full_pinyin_sentence() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let mut patterns = complete(&["wo", "xiang"]);
        patterns.push(vec![SyllablePattern::prefix("ka")]);
        assert_eq!(unigram(&dictionary, &patterns).unwrap().text, "我想开");
        // 全拼句子末尾的單字母多半是沒打完的音節，不參與
        let mut patterns = complete(&["wo", "xiang"]);
        patterns.push(vec![SyllablePattern::prefix("k")]);
        assert_eq!(unigram(&dictionary, &patterns).unwrap().text, "我想");
    }

    fn abbreviated<'a>(letters: &[&'a str]) -> Vec<Vec<SyllablePattern<'a>>> {
        letters
            .iter()
            .map(|s| vec![SyllablePattern::prefix(s)])
            .collect()
    }

    #[test]
    fn abbreviated_sentences_convert_by_prefix() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        // 全簡拼：每個格子按字首取詞，末尾單字母也是一個音節
        let conversion = unigram(&dictionary, &abbreviated(&["w", "x", "q"])).unwrap();
        assert_eq!(conversion.text, "我想去");
        assert_eq!(conversion.syllables, ["wo", "xiang", "qu"]);
        assert_eq!(conversion.words[0].text, "我想");
        // 簡拼與全拼混用
        let patterns = [
            vec![SyllablePattern::prefix("w")],
            vec![SyllablePattern::complete("xiang")],
            vec![SyllablePattern::prefix("q")],
            vec![SyllablePattern::prefix("c")],
            vec![SyllablePattern::prefix("f")],
        ];
        assert_eq!(unigram(&dictionary, &patterns).unwrap().text, "我想去吃饭");
        // 簡拼位置按字首取詞：`f` 既是 fa 也是 fang，詞頻高的 開放 勝出
        let conversion = unigram(&dictionary, &abbreviated(&["k", "f"])).unwrap();
        assert_eq!(conversion.text, "开放");
        assert!(!conversion.has_placeholder());
    }

    /// 只認 `我 → 翔` 的假模型下，簡拼 `w x` 也該聽語言模型的。
    #[test]
    fn language_model_decides_abbreviated_homophones() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = abbreviated(&["w", "x"]);
        let conversion = convert(
            &[&dictionary],
            &patterns,
            &XiangModel,
            Personal::NONE,
            |_| 0,
            |_, _| 0.0,
            &mut SpanCache::default(),
        )
        .unwrap();
        assert_eq!(conversion.text, "我翔");
    }

    #[test]
    fn unknown_syllables_are_kept_as_pinyin() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "zhuang", "qu"]);
        let conversion = unigram(&dictionary, &patterns).unwrap();
        assert_eq!(conversion.text, "我zhuang去");
        assert!(conversion.has_placeholder());
        assert!(conversion.words.iter().filter(|w| w.placeholder).count() == 1);
    }

    /// 只認 `我 → 翔` 的假模型：bigram 應該壓過一元詞頻。
    struct XiangModel;

    impl LanguageModel for XiangModel {
        fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
            match (previous, word) {
                (Some("我"), "翔") => Some(-2.0),
                (Some("我"), "想") => Some(-8.0),
                (None, "我") => Some(-1.0),
                _ => None,
            }
        }
    }

    #[test]
    fn language_model_decides_between_homophones() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "xiang"]);
        let conversion = convert(
            &[&dictionary],
            &patterns,
            &XiangModel,
            Personal::NONE,
            |_| 0,
            |_, _| 0.0,
            &mut SpanCache::default(),
        )
        .unwrap();
        assert_eq!(conversion.text, "我翔");
        assert_eq!(conversion.word_count(), 2);
    }

    #[test]
    fn personal_ngram_overrides_static_model_after_two_selections() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "xiang"]);
        let mut personal = UserNgram::default();
        let text = |personal: &UserNgram| {
            convert(
                &[&dictionary],
                &patterns,
                &XiangModel,
                Personal::new(Some(personal)),
                |_| 0,
                |_, _| 0.0,
                &mut SpanCache::default(),
            )
            .unwrap()
            .text
        };
        assert_eq!(text(&personal), "我翔");
        // 靜態模型給 翔 的是很強的 bigram（P ≈ 0.14）：使用者選過一次 我 → 想 翻不過（防誤選），兩次就翻。
        // 靜態證據越弱（P 越小），個人偏好翻過來得越早。
        personal.record(Context::START, "我");
        personal.record(Context::after("我"), "想");
        assert_eq!(text(&personal), "我翔");
        personal.record(Context::START, "我");
        personal.record(Context::after("我"), "想");
        assert_eq!(text(&personal), "我想");
    }

    /// 三元上下文來自前驅的回指：「我想」後面的 去 / 區 由使用者在「我想」後選過什麼決定，
    /// 而「臥想」後面（回指不同）拿不到這條三元。
    #[test]
    fn trigram_context_comes_from_the_predecessor_chain() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "xiang", "qu"]);
        let convert_with = |personal: &UserNgram| {
            convert(
                &[&dictionary],
                &patterns,
                &NoLanguageModel,
                Personal::new(Some(personal)),
                |_| 0,
                |_, _| 0.0,
                &mut SpanCache::default(),
            )
            .unwrap()
        };
        let mut personal = UserNgram::default();
        // 一元下 我想去（我想 是一個詞）
        assert_eq!(convert_with(&personal).text, "我想去");
        // 使用者在 我 → 想 之後選過 區：三元 (我, 想) → 區 把 區 抬過 去，路徑改走 我 / 想 / 區
        for _ in 0..4 {
            personal.record(Context::START, "我");
            personal.record(Context::after("我"), "想");
            personal.record(Context::after_two("我", "想"), "区");
        }
        let conversion = convert_with(&personal);
        assert_eq!(conversion.text, "我想区");
        assert_eq!(conversion.word_count(), 3);
    }

    #[test]
    fn cached_spans_give_the_same_sentence_and_only_new_spans_are_computed() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let mut cache = SpanCache::default();
        let run = |cache: &mut SpanCache, syllables: &[&str]| {
            let patterns = complete(syllables);
            convert(
                &[&dictionary],
                &patterns,
                &NoLanguageModel,
                Personal::NONE,
                |_| 0,
                |_, _| 0.0,
                cache,
            )
            .unwrap()
            .text
        };
        let fresh = run(&mut cache, &["wo", "xiang", "qu", "chi"]);
        let before = cache.len();
        // 多敲一個音節：只新增以它結尾的格子
        let extended = run(&mut cache, &["wo", "xiang", "qu", "chi", "fan"]);
        assert_eq!(extended, "我想去吃饭");
        assert!(cache.len() > before);
        assert!(cache.len() - before <= MAX_WORD_SYLLABLES);
        // 再算一遍全部命中快取，結果一致
        let again = cache.len();
        assert_eq!(run(&mut cache, &["wo", "xiang", "qu", "chi"]), fresh);
        assert_eq!(cache.len(), again);
    }

    /// 敲錯變體是帶代價的邊：`gan xi` 在 `gan` 位多一種寫法 `guan`（代價 4.5），原樣湊不出像樣的句子時 關係 勝出，
    /// 原樣本身說得通（`gan xie` 感謝）時代價讓它輸。
    #[test]
    fn typo_alternatives_are_penalized_edges() {
        let dictionary = Dictionary::parse(
            "关系\tguan xi\t500000\n干\tgan\t20000\n洗\txi\t10000\n感谢\tgan xie\t300000\n关\tguan\t30000\n谢\txie\t5000\n",
        )
        .unwrap();
        let cost = |index: usize, syllable: &str| {
            if index == 0 && syllable == "guan" {
                4.5
            } else {
                0.0
            }
        };
        let run = |positions: Vec<Vec<SyllablePattern<'_>>>| {
            convert(
                &[&dictionary],
                &positions,
                &NoLanguageModel,
                Personal::NONE,
                |_| 0,
                cost,
                &mut SpanCache::default(),
            )
            .unwrap()
        };
        let with_typo = |second: &'static str| {
            vec![
                vec![
                    SyllablePattern::complete("gan"),
                    SyllablePattern::complete("guan"),
                ],
                vec![SyllablePattern::complete(second)],
            ]
        };
        let conversion = run(with_typo("xi"));
        assert_eq!(conversion.text, "关系");
        assert_eq!(conversion.words[0].syllables, ["guan", "xi"]);
        // 原樣 乾洗 兩個單字的得分遠低於 關係 − 4.5：噪聲通道選糾正，路徑帶著代價
        assert_eq!(conversion.penalty, 4.5);
        let conversion = run(with_typo("xie"));
        assert_eq!(conversion.text, "感谢");
        assert_eq!(conversion.penalty, 0.0);
    }
}
