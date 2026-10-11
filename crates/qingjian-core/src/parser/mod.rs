//! 拼音切分：把無分隔的拼音串切成音節序列。
//!
//! 支援全拼、簡拼（聲母縮寫 `kf` → `k f`）與兩者混用（`kaif`、`kfa`），輸入允許用 `'` 強制分隔（`xi'an`）。
//! 末尾允許一個未打完的音節字首（`zho` → `zho…`）。每種切分裡每個音節標記是否完整，
//! 查詞時完整音節精確匹配、不完整音節按字首匹配。雙拼、模糊音後續在這裡擴充套件，介面保持不變。

mod error;
mod segmentation;
mod syllable;
mod trie;

use std::cmp::Reverse;

pub use error::ParseError;
pub use segmentation::{Segmentation, Syllable};
pub use syllable::{INITIALS, MAX_SYLLABLE_LEN, SYLLABLES};

use syllable::initial_lengths;
use trie::SYLLABLE_TRIE;

pub fn is_syllable(s: &str) -> bool {
    SYLLABLE_TRIE.contains(s)
}

/// `s` 是否為某個合法音節的**真**字首（不包含它自己就是完整音節的情況）。
pub fn is_syllable_prefix(s: &str) -> bool {
    SYLLABLE_TRIE.is_proper_prefix(s)
}

/// `text` 能否切成每個音節都完整的拼音。只回答是否，不產生切分、不分配音節：
/// 糾錯要對上千個變體逐個問這個問題，先用它過濾，剩下的幾個再做真正的切分。只認小寫字母。
pub fn is_fully_segmentable(text: &str) -> bool {
    let n = text.len();
    if n == 0 {
        return false;
    }
    let mut reachable = vec![false; n + 1];
    reachable[0] = true;
    for start in 0..n {
        if !reachable[start] {
            continue;
        }
        for len in SYLLABLE_TRIE.matches(&text[start..]).lengths() {
            reachable[start + len] = true;
        }
    }
    reachable[n]
}

/// 切分上限。簡拼讓歧義切分數量指數增長，每個位置只保留這麼多種最優切分。
const MAX_SEGMENTATIONS: usize = 8;

/// 切分。返回按「音節少、不完整音節少、前面的音節長」排序的切分，最多 [`MAX_SEGMENTATIONS`] 種。
pub fn segment(input: &str) -> Result<Vec<Segmentation>, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::Empty);
    }
    for (position, ch) in input.chars().enumerate() {
        if !(ch.is_ascii_lowercase() || ch == '\'') {
            return Err(ParseError::InvalidChar {
                position: position + 1,
                ch,
            });
        }
    }

    // 按 `'` 切成若干段，各段獨立切分後做笛卡爾積；只有最後一段允許殘缺音節。
    let chunks: Vec<&str> = input.split('\'').filter(|c| !c.is_empty()).collect();
    if chunks.is_empty() {
        return Err(ParseError::NoSegmentation);
    }
    let mut results: Vec<Segmentation> = vec![Segmentation {
        syllables: Vec::new(),
    }];
    for (index, chunk) in chunks.iter().enumerate() {
        let is_last = index + 1 == chunks.len();
        let options = segment_chunk(chunk, is_last);
        if options.is_empty() {
            return Err(ParseError::NoSegmentation);
        }
        let mut next = Vec::with_capacity(results.len() * options.len());
        for base in &results {
            for option in &options {
                let mut syllables = base.syllables.clone();
                syllables.extend(option.syllables.iter().cloned());
                next.push(Segmentation { syllables });
            }
        }
        results = next;
    }
    prune(&mut results);
    Ok(results)
}

/// 排序並截斷到 [`MAX_SEGMENTATIONS`]。
fn prune(segmentations: &mut Vec<Segmentation>) {
    segmentations.sort_by_cached_key(sort_key);
    segmentations.dedup();
    segmentations.truncate(MAX_SEGMENTATIONS);
}

/// 音節少的優先；同音節數時不完整音節少的優先；再同則前面的音節越長越優先（貪心的結果排最前）。
fn sort_key(segmentation: &Segmentation) -> (usize, usize, Vec<Reverse<usize>>) {
    (
        segmentation.syllables.len(),
        segmentation.incomplete_count(),
        segmentation
            .syllables
            .iter()
            .map(|s| Reverse(s.text.len()))
            .collect(),
    )
}

/// 切分不含 `'` 的一段：按位置做動態規劃，每個位置只保留最優的幾種字首切分。
/// `allow_partial` 為真時末尾允許留一個殘缺音節。
fn segment_chunk(chunk: &str, allow_partial: bool) -> Vec<Segmentation> {
    let n = chunk.len();
    let mut best: Vec<Vec<Segmentation>> = vec![Vec::new(); n + 1];
    best[0].push(Segmentation {
        syllables: Vec::new(),
    });
    for start in 0..n {
        if best[start].is_empty() {
            continue;
        }
        prune(&mut best[start]);
        let rest = &chunk[start..];
        let matches = SYLLABLE_TRIE.matches(rest);
        // (長度, 是否完整音節)
        let mut tokens: Vec<(usize, bool)> = matches.lengths().map(|len| (len, true)).collect();
        for len in initial_lengths(rest) {
            if !tokens.contains(&(len, true)) {
                tokens.push((len, false));
            }
        }
        // 整個剩餘部分作為未打完的音節（`zho` → zhong / zhou …）。它本身是完整音節或純聲母時已經在上面了。
        let rest_is_complete = matches.lengths().next_back() == Some(rest.len());
        if allow_partial
            && matches.whole_is_prefix
            && !rest_is_complete
            && !tokens.contains(&(rest.len(), false))
        {
            tokens.push((rest.len(), false));
        }
        for (len, complete) in tokens {
            let text = &rest[..len];
            let extended: Vec<Segmentation> = best[start]
                .iter()
                .map(|base| {
                    let mut syllables = base.syllables.clone();
                    syllables.push(if complete {
                        Syllable::complete(text)
                    } else {
                        Syllable::partial(text)
                    });
                    Segmentation { syllables }
                })
                .collect();
            best[start + len].extend(extended);
        }
    }
    let mut result = std::mem::take(&mut best[n]);
    prune(&mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(input: &str) -> Vec<String> {
        segment(input)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn greedy_segmentation_comes_first() {
        assert_eq!(joined("kaifa")[0], "kai fa");
        assert_eq!(joined("zhongwen")[0], "zhong wen");
    }

    #[test]
    fn ambiguous_input_yields_multiple_segmentations() {
        let all = joined("xian");
        assert_eq!(all[0], "xian");
        assert!(all.contains(&"xi an".to_owned()));
    }

    #[test]
    fn apostrophe_forces_boundary() {
        let all = joined("xi'an");
        assert_eq!(all[0], "xi an");
        assert!(!all.iter().any(|s| s.starts_with("xian")));
    }

    #[test]
    fn trailing_partial_syllable_is_marked() {
        assert_eq!(joined("kaif")[0], "kai f…");
        assert_eq!(joined("zho")[0], "zho…");
    }

    #[test]
    fn initials_only_segment_as_abbreviations() {
        assert_eq!(joined("kf")[0], "k… f…");
        assert_eq!(joined("zhw")[0], "zh… w…");
        assert!(joined("zhw").contains(&"z… h… w…".to_owned()));
    }

    #[test]
    fn mixed_full_and_abbreviated_syllables() {
        assert_eq!(joined("kfa")[0], "k… fa");
        assert_eq!(joined("kaif")[0], "kai f…");
        assert_eq!(joined("srf")[0], "s… r… f…");
    }

    #[test]
    fn complete_segmentation_ranks_before_partial() {
        let all = segment("xia").unwrap();
        assert_eq!(all[0].syllables, [Syllable::complete("xia")]);
    }

    #[test]
    fn long_input_stays_bounded() {
        let all = segment("womenjintianxiawuqukaihuiba").unwrap();
        assert!(all.len() <= MAX_SEGMENTATIONS);
        assert_eq!(all[0].to_string(), "wo men jin tian xia wu qu kai hui ba");
    }

    #[test]
    fn full_segmentability_agrees_with_segment() {
        for text in ["nihao", "zhongguo", "xian", "a", "kaifazhe"] {
            assert!(is_fully_segmentable(text), "{text}");
            assert_eq!(segment(text).unwrap()[0].incomplete_count(), 0, "{text}");
        }
        for text in ["", "nih", "kaif", "zhzh", "v", "nihoa"] {
            assert!(!is_fully_segmentable(text), "{text}");
        }
    }

    #[test]
    fn rejects_invalid_input() {
        assert_eq!(segment("").unwrap_err(), ParseError::Empty);
        assert_eq!(
            segment("kai1").unwrap_err(),
            ParseError::InvalidChar {
                position: 4,
                ch: '1'
            }
        );
        assert_eq!(segment("v").unwrap_err(), ParseError::NoSegmentation);
        assert_eq!(segment("kaiv").unwrap_err(), ParseError::NoSegmentation);
    }
}
