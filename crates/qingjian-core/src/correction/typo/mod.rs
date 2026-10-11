//! 音節級的敲錯變體：一個完整音節一處編輯（相鄰兩鍵換位、敲到相鄰鍵、多鍵、少鍵）之後還是合法音節的那些寫法。
//!
//! 詞圖裡每個完整音節除了敲的原樣，還按這些變體查詞，命中的詞扣掉相應代價（見 `TypoCosts`），
//! 讓「每個音節都合法、整句卻不通」的輸入（`meiganxi`）也能出 沒關係。整段一處編輯的糾錯（`variants`）管切不乾淨的輸入，兩者互補。
//!
//! 表按音節表一次算好：每個音節的變體幾十個，全部音節幾千條。

mod kind;

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::parser::{self, SYLLABLES};
pub use kind::TypoKind;

/// QWERTY 三排字母，算相鄰鍵用。
const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

/// 全部音節的敲錯變體：音節 → (變體, 類別)。同一變體只留代價最低的類別。
static TABLE: LazyLock<HashMap<&'static str, Vec<(String, TypoKind)>>> = LazyLock::new(|| {
    SYLLABLES
        .iter()
        .map(|syllable| (*syllable, compute_variants(syllable)))
        .collect()
});

/// `syllable` 的敲錯變體；不是完整音節（簡拼、沒打完的字首）或單字母音節沒有變體。
pub fn variants(syllable: &str) -> &'static [(String, TypoKind)] {
    TABLE.get(syllable).map_or(&[], Vec::as_slice)
}

/// `intended` 是不是 `typed` 的一處敲錯變體（讀取代價用 [`kind`]）。
pub fn is_variant(typed: &str, intended: &str) -> bool {
    kind(typed, intended).is_some()
}

/// 把 `typed` 敲成 `intended` 屬於哪類敲錯；不是一處敲錯返回 `None`。
pub fn kind(typed: &str, intended: &str) -> Option<TypoKind> {
    variants(typed)
        .iter()
        .find(|(text, _)| text == intended)
        .map(|(_, kind)| *kind)
}

/// 兩個字母在鍵盤上是否相鄰（同排隔壁，或上下排錯位挨著的兩個）。
pub fn adjacent(a: char, b: char) -> bool {
    let Some((row_a, col_a)) = position(a) else {
        return false;
    };
    let Some((row_b, col_b)) = position(b) else {
        return false;
    };
    if row_a == row_b {
        return col_a.abs_diff(col_b) == 1;
    }
    if row_a.abs_diff(row_b) != 1 {
        return false;
    }
    // 下一排整體向右錯開半個鍵：上排第 i 個鍵的下面是下排第 i−1 與第 i 個
    let (upper, lower) = if row_a < row_b {
        (col_a, col_b)
    } else {
        (col_b, col_a)
    };
    lower == upper || lower + 1 == upper
}

fn position(letter: char) -> Option<(usize, usize)> {
    ROWS.iter()
        .enumerate()
        .find_map(|(row, keys)| keys.find(letter).map(|col| (row, col)))
}

/// 一個音節的全部一處編輯（換鍵只算相鄰鍵）裡仍是合法音節的那些。單字母音節（`a` / `e` / `o`）不算：一鍵之差就是另一個字，談不上敲錯。
fn compute_variants(syllable: &str) -> Vec<(String, TypoKind)> {
    let bytes = syllable.as_bytes();
    let n = bytes.len();
    let mut found: Vec<(String, TypoKind)> = Vec::new();
    if n < 2 {
        return found;
    }
    let mut push = |text: String, kind: TypoKind| {
        if text == syllable || !parser::is_syllable(&text) {
            return;
        }
        match found.iter_mut().find(|(t, _)| *t == text) {
            Some(existing) => {
                if kind.cost() < existing.1.cost() {
                    existing.1 = kind;
                }
            }
            None => found.push((text, kind)),
        }
    };
    for i in 0..n - 1 {
        if bytes[i] != bytes[i + 1] {
            let mut v = bytes.to_vec();
            v.swap(i, i + 1);
            push(String::from_utf8(v).expect("ascii"), TypoKind::Transpose);
        }
    }
    for i in 0..n {
        for letter in b'a'..=b'z' {
            if letter == bytes[i] || !adjacent(bytes[i] as char, letter as char) {
                continue;
            }
            let mut v = bytes.to_vec();
            v[i] = letter;
            push(String::from_utf8(v).expect("ascii"), TypoKind::Substitute);
        }
    }
    for i in 0..n {
        let mut v = bytes.to_vec();
        v.remove(i);
        push(String::from_utf8(v).expect("ascii"), TypoKind::Extra);
    }
    for i in 0..=n {
        for letter in b'a'..=b'z' {
            let mut v = bytes.to_vec();
            v.insert(i, letter);
            push(String::from_utf8(v).expect("ascii"), TypoKind::Missing);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_adjacency() {
        assert!(adjacent('n', 'm'));
        assert!(adjacent('i', 'o'));
        assert!(adjacent('s', 'w'));
        assert!(adjacent('s', 'e'));
        assert!(adjacent('g', 'b'));
        assert!(!adjacent('a', 'k'));
        assert!(!adjacent('q', 'z'));
        assert!(!adjacent('a', 'a'));
    }

    #[test]
    fn variants_are_legal_syllables_one_edit_away() {
        let gan: Vec<&str> = variants("gan").iter().map(|(t, _)| t.as_str()).collect();
        assert!(gan.contains(&"guan"));
        assert!(gan.contains(&"gang"));
        assert!(gan.contains(&"an"));
        assert!(gan.contains(&"han")); // g 與 h 相鄰
        assert!(!gan.contains(&"dan")); // g 與 d 不相鄰
        assert!(!gan.contains(&"gan"));
        assert!(gan.iter().all(|t| parser::is_syllable(t)));
        assert_eq!(kind("gan", "guan"), Some(TypoKind::Missing));
        assert_eq!(kind("gan", "gang"), Some(TypoKind::Missing));
        assert_eq!(kind("gang", "gan"), Some(TypoKind::Extra));
        assert_eq!(kind("shou", "shuo"), Some(TypoKind::Transpose));
        assert_eq!(kind("ni", "mi"), Some(TypoKind::Substitute));
        assert_eq!(kind("ta", "da"), None);
        assert_eq!(kind("gan", "xi"), None);
        // 單字母音節、簡拼、字首沒有變體
        assert!(variants("a").is_empty());
        assert!(variants("zh").is_empty());
        assert!(variants("xia").len() >= 3); // xian / xiao / xi
        assert!(!is_variant("k", "ka"));
    }

    /// 同一變體能由多種編輯得到時留代價最低的：`shuo` → `shou` 既是換位也是兩處換鍵，算換位。
    #[test]
    fn keeps_the_cheapest_kind_per_variant() {
        assert_eq!(kind("shuo", "shou"), Some(TypoKind::Transpose));
    }
}
