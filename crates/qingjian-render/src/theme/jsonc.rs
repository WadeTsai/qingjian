//! JSONC：主題檔案可以寫 `//` 與 `/* */` 註釋、物件與陣列末尾多一個逗號（同 VS Code 的 `settings.json`）。
//! 解析前把註釋與多餘的逗號換成空格（換行照留），再交給 serde_json；位置不變，報錯的行號、列號仍對得上原檔案。

/// 去掉註釋與尾逗號。字串裡的 `//`、`,` 原樣保留；沒收尾的塊註釋吞到檔案末尾，交給 JSON 解析報錯。
pub(super) fn strip(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => i = skip_string(bytes, i),
            b'/' if matches!(bytes.get(i + 1), Some(b'/' | b'*')) => {
                let end = comment_end(bytes, i);
                blank(&mut out[i..end]);
                i = end;
            }
            b',' => {
                if matches!(next_significant(bytes, i + 1), Some(b'}' | b']')) {
                    out[i] = b' ';
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    // 換掉的是整個註釋（字元邊界上起止）或單個逗號，結果仍是合法 UTF-8
    String::from_utf8(out).unwrap_or_else(|_| source.to_owned())
}

/// `start` 是開引號，返回收引號之後的位置。
fn skip_string(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// `start` 是註釋的 `/`，返回註釋之後的位置（行註釋停在換行前）。
fn comment_end(bytes: &[u8], start: usize) -> usize {
    if bytes[start + 1] == b'/' {
        bytes[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |n| start + n)
    } else {
        bytes[start + 2..]
            .windows(2)
            .position(|pair| pair == b"*/")
            .map_or(bytes.len(), |n| start + 2 + n + 2)
    }
}

/// 從 `i` 起跳過空白與註釋後的第一個位元組。
fn next_significant(bytes: &[u8], mut i: usize) -> Option<u8> {
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'/' if matches!(bytes.get(i + 1), Some(b'/' | b'*')) => i = comment_end(bytes, i),
            other => return Some(other),
        }
    }
    None
}

/// 換成空格，換行照留；多位元組字元的每個位元組都換，結果仍是合法 UTF-8，位元組位置不變。
fn blank(bytes: &mut [u8]) {
    for byte in bytes {
        if *byte != b'\n' && *byte != b'\r' {
            *byte = b' ';
        }
    }
}

#[cfg(test)]
mod tests {
    use super::strip;

    fn parse(source: &str) -> serde_json::Value {
        serde_json::from_str(&strip(source)).unwrap()
    }

    #[test]
    fn comments_and_trailing_commas_are_dropped() {
        let value = parse(
            r#"{
                // 行注释
                "a": [1, 2, /* 块注释 */ 3,],
                /* 跨行
                   注释，里面有 "引号" 和 , */
                "b": { "c": "d", // 逗号后面是注释
                },
            }"#,
        );
        assert_eq!(
            value,
            serde_json::json!({ "a": [1, 2, 3], "b": { "c": "d" } })
        );
    }

    #[test]
    fn strings_are_left_alone() {
        let value = parse(r#"{ "url": "https://qingjian.app/a,]", "q": "say \"//hi\",}" }"#);
        assert_eq!(value["url"], "https://qingjian.app/a,]");
        assert_eq!(value["q"], "say \"//hi\",}");
    }

    #[test]
    fn errors_keep_original_positions() {
        let source = "{\n  // 注释\n  \"a\": 1,\n  \"b\": oops\n}";
        let error = serde_json::from_str::<serde_json::Value>(&strip(source)).unwrap_err();
        assert_eq!(error.line(), 4);
    }

    #[test]
    fn comment_markers_inside_comments_and_non_ascii_survive() {
        let value = parse("{ \"名字\": \"樱花\" /* 中文注释 // 套着 */ }");
        assert_eq!(value["名字"], "樱花");
    }
}
