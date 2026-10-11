/// 一處編輯：把使用者敲的串變成糾正後的串。下標是**糾正後**串裡的位置（純 ASCII 小寫，位元組即字元）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// 原串這一位敲的是 `from`，糾正後換成了別的字母。
    Substitute { index: usize, from: char },

    /// 原串在這一位多敲了 `removed`，糾正後沒有它（`index` 是它在糾正後串裡本該在的位置）。
    Delete { index: usize, removed: char },

    /// 原串漏了一個字母，糾正後在 `index` 處多出一個。
    Insert { index: usize },

    /// 原串這一位與下一位敲反了（原來是 `first``second`，糾正後是 `second``first`）。
    Transpose {
        index: usize,
        first: char,
        second: char,
    },
}

impl Edit {
    /// 糾正後串開頭 `corrected_len` 個字母對應原串開頭多少個字母：候選按糾正後的音節消耗拼音，
    /// 消耗掉的原串長度要按這處編輯換算回去。
    pub fn to_original(&self, corrected_len: usize) -> usize {
        match *self {
            Self::Substitute { .. } | Self::Transpose { .. } => corrected_len,
            // 多敲的字母緊貼在消耗掉的部分後面時一併吃掉，別把它留給下一段
            Self::Delete { index, .. } if index <= corrected_len => corrected_len + 1,
            Self::Delete { .. } => corrected_len,
            Self::Insert { index } if index < corrected_len => corrected_len - 1,
            Self::Insert { .. } => corrected_len,
        }
    }

    /// 這處編輯是不是改在剛敲的最後一個鍵上：換掉最後一個字母、或在末尾補一個字母（`corrected_len` 是糾正後串的長度）。
    /// 使用者多半還沒敲完，這種編輯湊出的完整拼音說明不了什麼。
    pub fn touches_last_letter(&self, corrected_len: usize) -> bool {
        match *self {
            Self::Substitute { index, .. } | Self::Insert { index } => index + 1 == corrected_len,
            Self::Delete { .. } | Self::Transpose { .. } => false,
        }
    }

    /// 要畫刪除線的原字母及其在糾正後串裡的位置（畫在這一位之前）；漏字沒有可劃的。
    pub fn struck(&self) -> Option<(usize, String)> {
        match *self {
            Self::Substitute { index, from } => Some((index, from.to_string())),
            Self::Delete { index, removed } => Some((index, removed.to_string())),
            Self::Insert { .. } => None,
            Self::Transpose {
                index,
                first,
                second,
            } => Some((index, format!("{first}{second}"))),
        }
    }
}

/// `input` 的全部一處編輯變體：先換位，再替換、刪除、插入。`input` 必須是純小寫字母。
pub fn variants(input: &str) -> Vec<(Edit, String)> {
    let bytes = input.as_bytes();
    let n = bytes.len();
    let mut out = Vec::with_capacity(n * 55);
    for i in 0..n.saturating_sub(1) {
        if bytes[i] != bytes[i + 1] {
            let mut v = bytes.to_vec();
            v.swap(i, i + 1);
            out.push((
                Edit::Transpose {
                    index: i,
                    first: bytes[i] as char,
                    second: bytes[i + 1] as char,
                },
                String::from_utf8(v).expect("ascii"),
            ));
        }
    }
    for i in 0..n {
        for letter in b'a'..=b'z' {
            if letter == bytes[i] {
                continue;
            }
            let mut v = bytes.to_vec();
            v[i] = letter;
            out.push((
                Edit::Substitute {
                    index: i,
                    from: bytes[i] as char,
                },
                String::from_utf8(v).expect("ascii"),
            ));
        }
    }
    for i in 0..n {
        let mut v = bytes.to_vec();
        v.remove(i);
        out.push((
            Edit::Delete {
                index: i,
                removed: bytes[i] as char,
            },
            String::from_utf8(v).expect("ascii"),
        ));
    }
    for i in 0..=n {
        for letter in b'a'..=b'z' {
            let mut v = bytes.to_vec();
            v.insert(i, letter);
            out.push((
                Edit::Insert { index: i },
                String::from_utf8(v).expect("ascii"),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_consumed_length_back_to_the_original() {
        assert_eq!(
            Edit::Substitute {
                index: 3,
                from: 'o'
            }
            .to_original(5),
            5
        );
        // 原 nihooma（7）→ 糾正 nihoma？這裡只看換算：刪掉下標 3 的字母，消耗 3 個糾正後字母時把它一起吃掉
        let delete = Edit::Delete {
            index: 3,
            removed: 'o',
        };
        assert_eq!(delete.to_original(2), 2);
        assert_eq!(delete.to_original(3), 4);
        assert_eq!(delete.to_original(6), 7);
        let insert = Edit::Insert { index: 3 };
        assert_eq!(insert.to_original(3), 3);
        assert_eq!(insert.to_original(4), 3);
        assert_eq!(insert.to_original(7), 6);
    }

    #[test]
    fn generates_every_single_edit_once() {
        let all = variants("ab");
        let texts: Vec<&str> = all.iter().map(|(_, t)| t.as_str()).collect();
        assert!(texts.contains(&"ba"));
        assert!(texts.contains(&"ac"));
        assert!(texts.contains(&"a"));
        assert!(texts.contains(&"zab"));
        // 1 換位 + 2×25 替換 + 2 刪除 + 3×26 插入
        assert_eq!(all.len(), 1 + 50 + 2 + 78);
        assert!(
            variants("aa")
                .iter()
                .all(|(e, _)| !matches!(e, Edit::Transpose { .. }))
        );
    }
}
