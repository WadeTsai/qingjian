//! 把一段漢字文本按語言模型切成詞序列（沒有拼音的場合，如接受了雲端的整句補全之後要記個人 n-gram）。
//!
//! 與拼音驅動的整句轉換不同，這裡的詞圖節點是文本子串：每個位置試 1 到 [`MAX_TEXT_WORD_CHARS`] 個字，
//! 模型認識的子串才算詞，單字總可以切出來（模型不認識就扣分）。Viterbi 狀態是「切到哪、上一個詞是什麼」，
//! 打分只用靜態模型（個人 bigram 還沒記這句，用它反而會自我強化）。標點等非漢字處斷句，各句獨立。

use super::LanguageModel;

/// 文本切詞時一個詞最多幾個字。
pub const MAX_TEXT_WORD_CHARS: usize = 6;

/// 模型不認識的單字的分數：相當於億級語料裡只出現一次的詞，任何認識的詞都比它強。
const UNKNOWN_CHAR_LOG_PROB: f64 = -18.5;

/// 切成一句句（非漢字處斷開）的詞序列。模型一個詞都不認識時返回 `None`：沒有語言模型（或全是生僻字）時切出來的
/// 只是單字流，記進個人 n-gram 沒有意義。
pub fn segment_text(text: &str, model: &dyn LanguageModel) -> Option<Vec<Vec<String>>> {
    let mut clauses = Vec::new();
    let mut known = false;
    for clause in text.split(|c: char| !is_han(c)) {
        let chars: Vec<char> = clause.chars().collect();
        if chars.is_empty() {
            continue;
        }
        let (words, any_known) = segment_clause(&chars, model);
        known |= any_known;
        clauses.push(words);
    }
    (known && !clauses.is_empty()).then_some(clauses)
}

/// 一句裡的 Viterbi：節點 = (起點, 詞)，每個節點記最優前驅；同一起點、同一個詞只留一個節點，
/// 前驅按「上一個詞」區分打分。返回詞序列和「有沒有模型認識的詞」。
fn segment_clause(chars: &[char], model: &dyn LanguageModel) -> (Vec<String>, bool) {
    struct Node {
        word: String,
        score: f64,
        known: bool,
        previous: Option<usize>,
    }
    let n = chars.len();
    let mut nodes: Vec<Node> = Vec::new();
    // 每個結束位置上的節點下標
    let mut ending: Vec<Vec<usize>> = vec![Vec::new(); n + 1];
    for start in 0..n {
        let predecessors: Vec<Option<usize>> = if start == 0 {
            vec![None]
        } else {
            ending[start].iter().map(|i| Some(*i)).collect()
        };
        if predecessors.is_empty() {
            continue;
        }
        for len in 1..=MAX_TEXT_WORD_CHARS.min(n - start) {
            let word: String = chars[start..start + len].iter().collect();
            let mut best: Option<Node> = None;
            for predecessor in &predecessors {
                let (previous, base) = match predecessor {
                    Some(i) => (Some(nodes[*i].word.as_str()), nodes[*i].score),
                    None => (None, 0.0),
                };
                let (log_prob, known) = match model.log_prob(previous, &word) {
                    Some(p) => (p, true),
                    None if len == 1 => (UNKNOWN_CHAR_LOG_PROB, false),
                    None => continue,
                };
                let score = base + log_prob;
                if best.as_ref().is_none_or(|b| score > b.score) {
                    best = Some(Node {
                        word: word.clone(),
                        score,
                        known,
                        previous: *predecessor,
                    });
                }
            }
            if let Some(node) = best {
                nodes.push(node);
                ending[start + len].push(nodes.len() - 1);
            }
        }
    }
    let Some(&last) = ending[n]
        .iter()
        .max_by(|a, b| nodes[**a].score.total_cmp(&nodes[**b].score))
    else {
        return (Vec::new(), false);
    };
    let mut path = Vec::new();
    let mut known = false;
    let mut cursor = Some(last);
    while let Some(i) = cursor {
        let node = &nodes[i];
        known |= node.known;
        path.push(node.word.clone());
        cursor = node.previous;
    }
    path.reverse();
    (path, known)
}

/// 是不是漢字（CJK 統一表意文字各區）。
pub(crate) fn is_han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
}

#[cfg(test)]
mod tests {
    use super::super::NoLanguageModel;
    use super::*;

    struct TinyModel;

    impl LanguageModel for TinyModel {
        fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
            let unigram = match word {
                "开发" => -5.0,
                "输入法" => -6.0,
                "输入" => -5.5,
                "法" => -7.0,
                "开" | "发" | "输" | "入" => -8.0,
                "我们" => -4.0,
                "我" => -4.5,
                "们" => -9.0,
                _ => return None,
            };
            let bonus = match (previous, word) {
                (Some("开发"), "输入法") => 2.0,
                _ => 0.0,
            };
            Some(unigram + bonus)
        }
    }

    #[test]
    fn picks_the_best_scoring_word_path() {
        let clauses = segment_text("开发输入法", &TinyModel).unwrap();
        assert_eq!(clauses, [vec!["开发", "输入法"]]);
    }

    #[test]
    fn punctuation_splits_clauses_and_unknown_chars_stay_single() {
        let clauses = segment_text("我们开发，输入法X。", &TinyModel).unwrap();
        assert_eq!(clauses, [vec!["我们", "开发"], vec!["输入法"]]);
        let clauses = segment_text("我们龘", &TinyModel).unwrap();
        assert_eq!(clauses, [vec!["我们", "龘"]]);
    }

    #[test]
    fn without_a_model_nothing_is_learned() {
        assert_eq!(segment_text("开发输入法", &NoLanguageModel), None);
        assert_eq!(segment_text("。。。", &TinyModel), None);
        assert_eq!(segment_text("", &TinyModel), None);
    }
}
