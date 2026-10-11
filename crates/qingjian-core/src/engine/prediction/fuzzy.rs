//! 雲端詞的拼音是否對得上使用者敲的字母：允許簡拼、少量錯字 / 漏字 / 多字。
//!
//! 模型的價值之一就是糾錯（`zhgdoima` → 這個東西嗎），所以不能要求逐音節精確相符；
//! 但也不能模型說什麼就是什麼，否則一個無關的詞會插進候選。這裡算「使用者的字母」與「該詞全拼」之間的編輯距離，
//! 其中每個音節允許只取字首（簡拼不算錯），超過容許的錯誤數就丟。

/// 使用者字母 `typed` 匹配到全拼音節 `syllables` 所需的最少錯誤數（替換 / 多敲 / 漏敲各算 1）。
/// 每個音節可以只匹配它的一個非空字首，省掉的字尾不算錯。
pub fn mismatch_count(typed: &str, syllables: &[String]) -> usize {
    let typed: Vec<u8> = typed.bytes().filter(|b| *b != b'\'').collect();
    let n = typed.len();
    const INF: usize = usize::MAX / 2;
    // dp[i]：typed 的前 i 個字母匹配完前面所有音節的最少錯誤數
    let mut dp = vec![INF; n + 1];
    dp[0] = 0;
    for syllable in syllables {
        let s = syllable.as_bytes();
        let mut next = vec![INF; n + 1];
        // 音節的每個非空字首都是一種可能的寫法；字首 p 與 typed[i..j] 的編輯距離經典 DP
        for start in 0..=n {
            if dp[start] == INF {
                continue;
            }
            // row[k][j-start]：字首長度 k 與 typed[start..j] 的編輯距離
            let mut row: Vec<usize> = (0..=n - start).collect();
            for k in 1..=s.len() {
                let mut new_row = vec![0; n - start + 1];
                new_row[0] = k;
                for j in 1..=n - start {
                    let cost = usize::from(s[k - 1] != typed[start + j - 1]);
                    new_row[j] = (row[j - 1] + cost).min(row[j] + 1).min(new_row[j - 1] + 1);
                }
                row = new_row;
                // 到這裡字首長度 k 已完整，可作為該音節的寫法收尾
                for j in 1..=n - start {
                    let total = dp[start] + row[j];
                    if total < next[start + j] {
                        next[start + j] = total;
                    }
                }
            }
        }
        dp = next;
    }
    dp[n]
}

/// 容許的錯誤數：不到 4 個字母不容錯（`zt` 容 1 個錯就什麼詞都對得上），4 個起容 1 個，之後每 6 個字母多容 1 個。
pub fn tolerance(letters: usize) -> usize {
    if letters < 4 {
        0
    } else {
        1 + (letters - 4) / 6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syl(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn exact_and_abbreviated_pinyin_cost_nothing() {
        assert_eq!(mismatch_count("zhangtao", &syl(&["zhang", "tao"])), 0);
        assert_eq!(mismatch_count("zt", &syl(&["zhang", "tao"])), 0);
        assert_eq!(
            mismatch_count("zh'g'do'ima", &syl(&["zhe", "ge", "dong", "xi", "ma"])),
            1
        );
        assert_eq!(mismatch_count("kaifa", &syl(&["kai", "fa"])), 0);
    }

    #[test]
    fn typos_are_counted_and_unrelated_words_rejected() {
        // 錯一個字母
        assert_eq!(mismatch_count("kaufa", &syl(&["kai", "fa"])), 1);
        // 多敲一個
        assert_eq!(mismatch_count("kaifaa", &syl(&["kai", "fa"])), 1);
        // 完全無關
        assert!(mismatch_count("zhangtao", &syl(&["zhi", "dao"])) >= 3);
        assert!(mismatch_count("zt", &syl(&["kai", "fa"])) >= 2);
        // 音節數不對也會體現在錯誤數上
        assert!(mismatch_count("zhangtao", &syl(&["zhang", "tao", "tao"])) >= 1);
    }

    #[test]
    fn tolerance_grows_slowly_with_length() {
        assert_eq!(tolerance(2), 0);
        assert_eq!(tolerance(3), 0);
        assert_eq!(tolerance(5), 1);
        assert_eq!(tolerance(9), 1);
        assert_eq!(tolerance(10), 2);
    }
}
