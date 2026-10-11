use std::sync::LazyLock;

use super::syllable::{MAX_SYLLABLE_LEN, SYLLABLES};

/// 全域性音節 trie，首次使用時從 [`SYLLABLES`] 構建。
pub static SYLLABLE_TRIE: LazyLock<SyllableTrie> = LazyLock::new(|| SyllableTrie::build(SYLLABLES));

/// trie 節點。子節點按字母 a–z 索引，0 表示無子節點（根節點永遠是 0 號，不會被引用為子節點）。
#[derive(Debug, Clone)]
struct Node {
    /// 26 個字母對應的子節點下標。
    children: [u16; 26],

    /// 從根到此節點的路徑是否為一個完整音節。
    terminal: bool,
}

impl Node {
    const EMPTY: Self = Self {
        children: [0; 26],
        terminal: false,
    };
}

/// 從輸入某個位置起的匹配結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matches {
    /// 以該位置開頭的合法音節長度，升序；最多 [`MAX_SYLLABLE_LEN`] 個。
    lengths: [u8; MAX_SYLLABLE_LEN],

    /// `lengths` 裡有效的個數。
    count: u8,

    /// 剩餘整段是某個更長音節的真字首（用於末尾殘缺音節）。
    pub whole_is_prefix: bool,
}

impl Matches {
    pub fn lengths(&self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.lengths[..usize::from(self.count)]
            .iter()
            .map(|&len| usize::from(len))
    }
}

/// 小寫字母 trie，只接受 a–z。
#[derive(Debug)]
pub struct SyllableTrie {
    /// 節點池，0 號為根。
    nodes: Vec<Node>,
}

impl SyllableTrie {
    pub fn build(syllables: &[&str]) -> Self {
        let mut trie = Self {
            nodes: vec![Node::EMPTY],
        };
        for syllable in syllables {
            let mut current = 0usize;
            for byte in syllable.bytes() {
                let slot = Self::slot(byte).expect("音节表只能含小写字母");
                let next = trie.nodes[current].children[slot];
                current = if next == 0 {
                    trie.nodes.push(Node::EMPTY);
                    let index = trie.nodes.len() - 1;
                    trie.nodes[current].children[slot] =
                        u16::try_from(index).expect("音节 trie 节点数不会超过 u16");
                    index
                } else {
                    usize::from(next)
                };
            }
            trie.nodes[current].terminal = true;
        }
        trie
    }

    fn slot(byte: u8) -> Option<usize> {
        byte.is_ascii_lowercase().then(|| usize::from(byte - b'a'))
    }

    /// 沿 `rest` 從根走到走不動為止，收集途中經過的完整音節。
    pub fn matches(&self, rest: &str) -> Matches {
        let mut result = Matches {
            lengths: [0; MAX_SYLLABLE_LEN],
            count: 0,
            whole_is_prefix: false,
        };
        let mut current = 0usize;
        for (index, byte) in rest.bytes().enumerate() {
            let Some(slot) = Self::slot(byte) else {
                return result;
            };
            let next = self.nodes[current].children[slot];
            if next == 0 {
                return result;
            }
            current = usize::from(next);
            if self.nodes[current].terminal {
                result.lengths[usize::from(result.count)] =
                    u8::try_from(index + 1).expect("音节长度不超过 6");
                result.count += 1;
            }
        }
        // 整段走完了還有子節點，說明它是某個更長音節的真字首
        result.whole_is_prefix =
            !rest.is_empty() && self.nodes[current].children.iter().any(|&c| c != 0);
        result
    }

    pub fn contains(&self, s: &str) -> bool {
        self.matches(s).lengths().next_back() == Some(s.len()) && !s.is_empty()
    }

    /// `s` 是否為某個合法音節的**真**字首（不包含它自己就是完整音節的情況）。
    pub fn is_proper_prefix(&self, s: &str) -> bool {
        self.matches(s).whole_is_prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_matches_table_exactly() {
        for syllable in SYLLABLES {
            assert!(SYLLABLE_TRIE.contains(syllable), "{syllable}");
        }
        assert!(!SYLLABLE_TRIE.contains(""));
        assert!(!SYLLABLE_TRIE.contains("v"));
        assert!(!SYLLABLE_TRIE.contains("kaif"));
        assert!(!SYLLABLE_TRIE.contains("Kai"));
    }

    #[test]
    fn matches_lists_all_syllable_lengths_ascending() {
        let lengths: Vec<usize> = SYLLABLE_TRIE.matches("xiangmu").lengths().collect();
        assert_eq!(lengths, [2, 3, 4, 5]); // xi / xia / xian / xiang
        assert!(!SYLLABLE_TRIE.matches("xiangmu").whole_is_prefix);
    }

    #[test]
    fn proper_prefix_excludes_complete_syllables() {
        assert!(SYLLABLE_TRIE.is_proper_prefix("zh"));
        assert!(SYLLABLE_TRIE.is_proper_prefix("xia"));
        assert!(!SYLLABLE_TRIE.is_proper_prefix("zhuang"));
        assert!(!SYLLABLE_TRIE.is_proper_prefix("v"));
        assert!(!SYLLABLE_TRIE.is_proper_prefix(""));
    }

    #[test]
    fn agrees_with_linear_scan_on_every_prefix() {
        for syllable in SYLLABLES {
            for end in 1..=syllable.len() {
                let s = &syllable[..end];
                let linear_prefix = SYLLABLES
                    .iter()
                    .any(|t| t.len() > s.len() && t.starts_with(s));
                assert_eq!(SYLLABLE_TRIE.is_proper_prefix(s), linear_prefix, "{s}");
                assert_eq!(SYLLABLE_TRIE.contains(s), SYLLABLES.contains(&s), "{s}");
            }
        }
    }
}
