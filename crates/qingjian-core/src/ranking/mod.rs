//! 候選排序。
//!
//! 詞級排序規則：
//! 1. 音節數與輸入完全一致的詞優先（`kaifa` → 開發 排在 開發者 前）
//! 2. 覆蓋輸入字母多者優先（`kaif` → 開發者 排在 開 前）
//! 3. 切分裡非末尾的簡拼音節少者優先（`kaifa` 按 `kai fa` 讀的 開放 排在按 `kai f a` 讀的 開放啊 前）
//! 4. 最後一個音節完整匹配優先（`kaifa` → 開發 排在 開放 前）
//! 5. 同一輸入串下使用者選過的次數（Learner 的 `choice_weight`：`mgs` 選過 美國式，下次 `mgs` 它就是首選）
//! 6. 上下文得分：語言模型給的 `log P(詞 | 上一個上屏的詞)`（個人 bigram 插值，模型不認識的按詞庫詞頻兜底並扣分，
//!    見 `sentence::transition_log_prob`）加使用者選擇次數的加分（[`weight_bonus`]，對數且封頂），模糊音命中扣 ln 2、
//!    敲錯變體命中扣那類敲錯的代價（`correction::TypoKind::cost`，個人敲錯表打折）。
//!    這樣 `ba` 在「做了」後面出 吧、句首出 把；純詞頻排序兩處都只能出同一個
//! 7. 敲的原音節優先，詞長短者優先，最後按字串穩定排序保證結果可復現
//!
//! 詞庫靜態詞頻只用於預選（命中太多時先按詞頻砍到夠排的量）與兜底。

mod scored;

use std::collections::HashSet;

pub use scored::{PreselectKey, Scored, SortKey};

/// 使用者選擇次數的加分系數：加分 = 係數 × ln(1 + min(次數, [`WEIGHT_CAP`]))。
/// 這個加分只負責「同音詞裡偏向使用者常選的那個」，所以既取對數又封頂（最多約 1.5 分，e^1.5 ≈ 4.5 倍），
/// 讓上下文（bigram）仍能壓過它；不封頂時選過 173 次的 的 會帶著 5 分加分把任何含 的 的拆分路徑抬到整詞之上
/// （`haode` 出 號的 而不是 好的）。使用者用得多的詞另有個人 bigram 裡的一元項撐腰，不靠這裡。
pub const WEIGHT_BONUS: f64 = 0.5;

/// 加分裡計入的選擇次數上限。
pub const WEIGHT_CAP: u32 = 20;

/// 模糊音命中扣的分（詞頻減半）。敲錯變體的代價見 `correction::TypoKind`。
pub const FUZZY_PENALTY: f64 = std::f64::consts::LN_2;

/// 使用者選擇次數換算成得分加成，詞級排序與整句路徑共用，見 [`WEIGHT_BONUS`]。
pub fn weight_bonus(count: u32) -> f64 {
    WEIGHT_BONUS * (1.0 + f64::from(count.min(WEIGHT_CAP))).ln()
}

/// 排序並按詞文本去重（同一個詞可能被多種切分命中，保留得分最高的一條），最多留 `limit` 條。
/// `context` 給每條命中算（同輸入串下的選擇次數, 上下文 log 機率），只對預選後剩下的那些呼叫。
///
/// 排序鍵先算好再排：單字母簡拼能命中兩萬條，比較器裡每次數字符數會讓排序佔掉幾十毫秒；去重也只做到夠數為止。
pub fn rank(
    items: &mut Vec<Scored<'_>>,
    limit: usize,
    context: impl Fn(&Scored<'_>) -> (u32, f64),
) {
    // 遠超上限時先按結構鍵 + 詞頻線性選出前面一段：同一個詞會被多種切分命中，多選一倍留給去重（結果仍可能略少於上限，無妨）
    let preselect = limit.saturating_mul(2);
    if items.len() > preselect.saturating_mul(2) {
        let mut keyed: Vec<(PreselectKey, Scored<'_>)> = items
            .drain(..)
            .map(|item| (item.preselect_key(), item))
            .collect();
        keyed.select_nth_unstable_by(preselect, |a, b| b.0.cmp(&a.0));
        keyed.truncate(preselect);
        items.extend(keyed.into_iter().map(|(_, item)| item));
    }
    let mut keyed: Vec<(SortKey<'_>, Scored<'_>)> = items
        .drain(..)
        .map(|item| {
            let (choice, log_prob) = context(&item);
            let score = log_prob + weight_bonus(item.weight) - item.penalty;
            (item.key(choice, score), item)
        })
        .collect();
    keyed.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let mut seen: HashSet<&str> = HashSet::with_capacity(limit.min(keyed.len()));
    items.extend(
        keyed
            .into_iter()
            .map(|(_, item)| item)
            .filter(|item| seen.insert(item.hit.text))
            .take(limit),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_dictionary::Match;

    fn hit<'a>(text: &'a str, pinyin: &'a str, frequency: u32, exact: bool) -> Match<'a> {
        Match {
            text,
            pinyin,
            frequency,
            exact,
        }
    }

    #[test]
    fn exact_beats_frequency_and_weight_beats_static_frequency() {
        let mut items = vec![
            Scored {
                hit: hit("开发者", "kai fa zhe", 99999, false),
                full_last: true,
                coverage: 5,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
            Scored {
                hit: hit("开放", "kai fang", 20000, true),
                full_last: false,
                coverage: 5,
                abbreviated: 0,
                weight: 5,
                penalty: 0.0,
            },
            Scored {
                hit: hit("开发", "kai fa", 9000, true),
                full_last: true,
                coverage: 5,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
        ];
        rank(&mut items, usize::MAX, |_| (0, 0.0));
        let texts: Vec<&str> = items.iter().map(|s| s.hit.text).collect();
        assert_eq!(texts, ["开发", "开放", "开发者"]);
    }

    #[test]
    fn context_score_orders_within_the_same_structure() {
        let mut items = vec![
            Scored {
                hit: hit("把", "ba", 3_000_000, true),
                full_last: true,
                coverage: 2,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
            Scored {
                hit: hit("吧", "ba", 2_000_000, true),
                full_last: true,
                coverage: 2,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
        ];
        // 上下文說 吧 更像：詞頻高的 把 讓位
        let by_context = |s: &Scored<'_>| (0, if s.hit.text == "吧" { -1.0 } else { -6.0 });
        rank(&mut items, usize::MAX, by_context);
        let texts: Vec<&str> = items.iter().map(|s| s.hit.text).collect();
        assert_eq!(texts, ["吧", "把"]);
        // 同一輸入串下選過的壓過上下文
        rank(&mut items, usize::MAX, |s| {
            (
                u32::from(s.hit.text == "把"),
                if s.hit.text == "吧" { -1.0 } else { -6.0 },
            )
        });
        assert_eq!(items[0].hit.text, "把");
        // 同分時使用者選過的、非模糊音的靠前
        for item in &mut items {
            item.weight = u32::from(item.hit.text == "把") * 3;
        }
        rank(&mut items, usize::MAX, |_| (0, -2.0));
        assert_eq!(items[0].hit.text, "把");
        for item in &mut items {
            item.weight = 3;
            item.penalty = if item.hit.text == "把" {
                FUZZY_PENALTY
            } else {
                0.0
            };
        }
        rank(&mut items, usize::MAX, |_| (0, -2.0));
        assert_eq!(items[0].hit.text, "吧");
    }

    #[test]
    fn fewer_abbreviated_syllables_win_at_equal_coverage() {
        let mut items = vec![
            Scored {
                hit: hit("开放啊", "kai fang a", 132, true),
                full_last: true,
                coverage: 5,
                abbreviated: 1,
                weight: 0,
                penalty: 0.0,
            },
            Scored {
                hit: hit("开放", "kai fang", 500_000, true),
                full_last: false,
                coverage: 5,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
        ];
        rank(&mut items, usize::MAX, |_| (0, 0.0));
        assert_eq!(items[0].hit.text, "开放");
    }

    #[test]
    fn weight_bonus_is_logarithmic_and_capped() {
        assert_eq!(weight_bonus(0), 0.0);
        assert!(weight_bonus(1) < weight_bonus(10));
        assert_eq!(weight_bonus(WEIGHT_CAP), weight_bonus(WEIGHT_CAP * 50));
        assert!(weight_bonus(u32::MAX) < 1.6);
    }

    #[test]
    fn deduplicates_by_text_keeping_best() {
        let mut items = vec![
            Scored {
                hit: hit("西安", "xi an", 4000, false),
                full_last: true,
                coverage: 4,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
            Scored {
                hit: hit("西安", "xi an", 4000, true),
                full_last: true,
                coverage: 4,
                abbreviated: 0,
                weight: 0,
                penalty: 0.0,
            },
        ];
        rank(&mut items, usize::MAX, |_| (0, 0.0));
        assert_eq!(items.len(), 1);
        assert!(items[0].hit.exact);
    }
}
