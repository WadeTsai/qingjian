//! 兩個小寫串是否只差一處編輯（替換、插入、刪除、相鄰換位）。

/// `a` 與 `b` 相等，或只差一處編輯：換一個字母、多 / 少一個字母、相鄰兩個字母對調。
/// 英文模式拼錯一個字母時用它找詞，串都很短，逐位元組比就夠了。
pub fn within_one_edit(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    match a.len().abs_diff(b.len()) {
        0 => same_length(a, b),
        1 => {
            let (short, long) = if a.len() < b.len() { (a, b) } else { (b, a) };
            one_insertion(short, long)
        }
        _ => false,
    }
}

/// 等長：最多一個位置不同，或恰好相鄰兩個位置對調。
fn same_length(a: &[u8], b: &[u8]) -> bool {
    let mut differences = a.iter().zip(b).enumerate().filter(|(_, (x, y))| x != y);
    let Some((first, _)) = differences.next() else {
        return true;
    };
    match differences.next() {
        None => true,
        Some((second, _)) => {
            second == first + 1
                && a[first] == b[second]
                && a[second] == b[first]
                && differences.next().is_none()
        }
    }
}

/// `long` 去掉一個位元組後等於 `short`。
fn one_insertion(short: &[u8], long: &[u8]) -> bool {
    let mut i = 0;
    let mut skipped = false;
    for &byte in long {
        if i < short.len() && short[i] == byte {
            i += 1;
        } else if skipped {
            return false;
        } else {
            skipped = true;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_each_kind_of_edit() {
        assert!(within_one_edit("hello", "hello"));
        assert!(within_one_edit("hallo", "hello")); // 替換
        assert!(within_one_edit("helo", "hello")); // 少一個
        assert!(within_one_edit("helllo", "hello")); // 多一個
        assert!(within_one_edit("hlelo", "hello")); // 換位
        assert!(within_one_edit("ello", "hello")); // 開頭少一個
        assert!(within_one_edit("hell", "hello")); // 結尾少一個
    }

    #[test]
    fn rejects_two_edits() {
        assert!(!within_one_edit("hxllx", "hello"));
        assert!(!within_one_edit("hlleo", "hello"));
        assert!(!within_one_edit("he", "hello"));
        assert!(!within_one_edit("hello", "olleh"));
        assert!(!within_one_edit("abcd", "acbe"));
    }
}
