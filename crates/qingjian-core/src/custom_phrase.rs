//! 自定義短語配置及共享校驗、預覽；由 Core 匹配，平臺負責編輯與展示。

use serde::{Deserialize, Serialize};

/// 按原始輸入碼匹配的固定位置文本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPhrase {
    /// 小寫英文字母輸入碼，1–32 個字元。
    pub code: String,

    /// 原樣上屏的文本，保留空格和換行。
    pub text: String,

    /// 從 1 開始的固定候選位置，最大 9。
    pub position: usize,

    /// 是否啟用；停用規則仍保留位置。
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn enabled() -> bool {
    true
}

/// 儲存和載入使用同一校驗，不覆蓋衝突條目。
pub fn validate_phrases(phrases: &[CustomPhrase]) -> Result<(), String> {
    let mut occupied = std::collections::BTreeSet::new();
    for phrase in phrases {
        if !valid_code(&phrase.code) {
            return Err("輸入碼須為 1–32 個小寫英文字母".into());
        }
        if !(1..=9).contains(&phrase.position) {
            return Err("候選位置須為 1–9".into());
        }
        if phrase.text.is_empty() {
            return Err("自定義短語不能為空".into());
        }
        if !occupied.insert((&phrase.code, phrase.position)) {
            return Err(format!(
                "輸入碼 {} 的第 {} 位已被佔用，不能儲存",
                phrase.code, phrase.position
            ));
        }
    }
    Ok(())
}

/// 輸入碼能不能當自定義短語用：1–32 個小寫英文字母。
fn valid_code(code: &str) -> bool {
    !code.is_empty() && code.len() <= 32 && code.bytes().all(|c| c.is_ascii_lowercase())
}

/// 把系統的文本替換（macOS「鍵盤 → 文本替換」這類「輸入碼 → 短語」表）並進自定義短語：
/// 每條佔該輸入碼最靠前的空位（1–9），輸入碼不是小寫字母、短語為空、九位都滿、或使用者已有同碼同文本的規則時跳過。
/// 結果保證通過 [`validate_phrases`]（前提是 `base` 本身合法）。
pub fn merge_replacements<'a>(
    base: &[CustomPhrase],
    replacements: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<CustomPhrase> {
    let mut phrases = base.to_vec();
    for (code, text) in replacements {
        if !valid_code(code) || text.is_empty() {
            continue;
        }
        if phrases.iter().any(|p| p.code == code && p.text == text) {
            continue;
        }
        let Some(position) =
            (1..=9).find(|n| !phrases.iter().any(|p| p.code == code && p.position == *n))
        else {
            continue;
        };
        phrases.push(CustomPhrase {
            code: code.to_owned(),
            text: text.to_owned(),
            position,
            enabled: true,
        });
    }
    phrases
}

impl CustomPhrase {
    /// 單行預覽保留 Unicode 字元邊界，用可見符號表示換行和製表符。
    pub fn preview(text: &str, max_chars: usize) -> String {
        let mut chars = text.chars().peekable();
        let mut preview = String::new();
        for _ in 0..max_chars {
            let Some(c) = chars.next() else {
                break;
            };
            preview.push(match c {
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    '↵'
                }
                '\n' => '↵',
                '\t' => '⇥',
                _ => c,
            });
        }
        if chars.next().is_some() {
            preview.push('…');
        }
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::{CustomPhrase, merge_replacements, validate_phrases};

    fn phrase(code: &str, text: &str, position: usize) -> CustomPhrase {
        CustomPhrase {
            code: code.into(),
            text: text.into(),
            position,
            enabled: true,
        }
    }

    #[test]
    fn replacements_take_the_first_free_slot_of_their_code() {
        let base = vec![phrase("yx", "第一位", 1), phrase("ee", "：", 1)];
        let merged = merge_replacements(
            &base,
            [
                ("yx", "qi@example.com"),
                ("omw", "On my way!"),
                ("ee", "："),
                ("Gs", "大寫不算"),
                ("a1", "帶數字不算"),
                ("", "空碼不算"),
                ("kong", ""),
            ],
        );
        validate_phrases(&merged).unwrap();
        assert_eq!(merged.len(), 4);
        assert_eq!(merged[2], phrase("yx", "qi@example.com", 2));
        assert_eq!(merged[3], phrase("omw", "On my way!", 1));
    }

    #[test]
    fn replacements_stop_at_nine_slots_per_code() {
        let base: Vec<_> = (1..=9).map(|n| phrase("x", &n.to_string(), n)).collect();
        let merged = merge_replacements(&base, [("x", "第十條")]);
        assert_eq!(merged.len(), 9);
    }
}
