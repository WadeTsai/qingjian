//! 快捷候選（搜狗「v 模式」的那一套）：不查詞庫、由輸入本身直接算出來的候選。
//!
//! - `rq` / `sj` / `xq`：今天的日期、現在的時間、星期幾，插在本地候選第二位起。
//! - 表示式鍵（預設 `v`）開頭進表示式模式：`v1+2` 出 `3` 與 `1+2=3`，`v123` 出中文數字與金額（小寫與大寫），
//!   `v123.5` 出小數讀法與金額，同樣各有小寫與大寫（一百二十三元五角 / 壹佰貳拾叄元伍角）。
//!   表示式模式下緩衝區允許數字與運算子，候選不走拼音解析。
//! - 問字鍵（預設 `u`）後跟十六進位制碼點出那個字元：`u4e00` → 一，`u+1f600` → 😀。模式鍵本身在 `engine::ModeKeys`。

mod calendar;
mod evaluator;
mod numeral;

use jiff::Zoned;

use crate::candidate::{Candidate, CandidateKind};

pub use calendar::{date_forms, time_forms, weekday_forms};
pub use evaluator::evaluate;
pub use numeral::{
    amount_lower, amount_upper, chinese_decimal_lower, chinese_decimal_upper, chinese_lower,
    chinese_upper,
};

/// 表示式模式的預設字首。`v` 不是任何拼音音節的開頭，用它不會和拼音衝突。
pub const EXPRESSION_PREFIX: char = 'v';

/// 碼點最長几位十六進位制（U+10FFFF）。
const MAX_CODEPOINT_DIGITS: usize = 6;

/// 問字字首之後的部分是不是 Unicode 碼點：`4e00`、`+1f600`。全是十六進位制且至少含一個數字，
/// 或以 `+` 開頭（`+face` 這種純字母也算）；純字母不帶 `+` 當拼音問題（`fade` 是合法拼音）。
/// 是碼點就返回那個字元（不可顯示的控制字元、代理區、超範圍返回 `None`）。
pub fn unicode_form(body: &str) -> Option<String> {
    let (explicit, hex) = match body.strip_prefix('+') {
        Some(rest) => (true, rest),
        None => (false, body),
    };
    if hex.is_empty()
        || hex.len() > MAX_CODEPOINT_DIGITS
        || !hex.bytes().all(|b| b.is_ascii_hexdigit())
        || (!explicit && !hex.bytes().any(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    let c = char::from_u32(value)?;
    (!c.is_control()).then(|| c.to_string())
}

/// 到目前為止敲的還可能是碼點（空、`+`、或全是十六進位制）：殼據此決定數字進緩衝區還是選詞。
pub fn could_be_unicode(body: &str) -> bool {
    let hex = body.strip_prefix('+').unwrap_or(body);
    hex.len() <= MAX_CODEPOINT_DIGITS && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// 表示式模式下允許敲進緩衝區的非字母字元：數字與四則運算子號。
/// 字母（`x` 當乘號）本來就能進緩衝區，不在此列。
pub fn is_expression_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '+' | '-' | '*' | '/' | '(' | ')' | '.' | '^')
}

/// 按輸入算快捷候選；不是快捷輸入時為空。`expression` 是表示式鍵；`now` 由呼叫方給，測試可固定時間。
pub fn candidates(input: &str, expression: char, now: &Zoned) -> Vec<Candidate> {
    let texts: Vec<String> = match input {
        "rq" => date_forms(now),
        "sj" => time_forms(now),
        "xq" => weekday_forms(now),
        _ => match input.strip_prefix(expression) {
            Some(body) => expression_forms(body),
            None => Vec::new(),
        },
    };
    texts
        .into_iter()
        .map(|text| Candidate {
            text,
            kind: CandidateKind::Shortcut,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        })
        .collect()
}

/// 金額最多幾位小數（角、分）。
const AMOUNT_FRACTION_DIGITS: usize = 2;

/// 表示式鍵之後的部分：一個數出中文數字與金額，四則運算出結果與「算式=結果」。
fn expression_forms(body: &str) -> Vec<String> {
    if let Some(forms) = number_forms(body) {
        return forms;
    }
    match evaluate(body) {
        Some(result) => vec![result.clone(), format!("{body}={result}")],
        None => Vec::new(),
    }
}

/// 單個非負數（`123`、`123.5`）：中文數字在前，金額在後；小數超過兩位沒有金額寫法。
/// 末尾剛敲了 `.`（`123.`）按整數給，候選不閃。
fn number_forms(body: &str) -> Option<Vec<String>> {
    let (integer, fraction) = body.split_once('.').unwrap_or((body, ""));
    let digits = |text: &str| text.bytes().all(|b| b.is_ascii_digit());
    if integer.is_empty() || !digits(integer) || !digits(fraction) {
        return None;
    }
    let mut forms = if fraction.is_empty() {
        vec![chinese_lower(integer), chinese_upper(integer)]
    } else {
        vec![
            chinese_decimal_lower(integer, fraction),
            chinese_decimal_upper(integer, fraction),
        ]
    };
    if fraction.len() <= AMOUNT_FRACTION_DIGITS {
        forms.push(amount_lower(integer, fraction));
        forms.push(amount_upper(integer, fraction));
    }
    Some(forms)
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use super::*;

    #[test]
    fn unicode_bodies_need_a_digit_or_a_plus() {
        assert_eq!(unicode_form("4e00").as_deref(), Some("一"));
        assert_eq!(unicode_form("+1f600").as_deref(), Some("😀"));
        assert_eq!(unicode_form("+face").as_deref(), Some("\u{face}"));
        // 純字母沒有 + 當拼音問題（fade 是合法拼音）
        assert_eq!(unicode_form("fade"), None);
        assert_eq!(unicode_form(""), None);
        assert_eq!(unicode_form("+"), None);
        // 控制字元、代理區、超範圍
        assert_eq!(unicode_form("07"), None);
        assert_eq!(unicode_form("d800"), None);
        assert_eq!(unicode_form("110000"), None);
        assert_eq!(unicode_form("1234567"), None);
        assert!(could_be_unicode(""));
        assert!(could_be_unicode("+"));
        assert!(could_be_unicode("4e"));
        assert!(could_be_unicode("face"));
        assert!(!could_be_unicode("sangemu"));
    }

    fn now() -> Zoned {
        date(2026, 9, 3)
            .at(19, 6, 23, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
    }

    fn texts(input: &str) -> Vec<String> {
        candidates(input, EXPRESSION_PREFIX, &now())
            .into_iter()
            .map(|c| c.text)
            .collect()
    }

    #[test]
    fn calendar_shortcuts() {
        assert_eq!(texts("rq"), ["2026年9月3日", "2026-09-03", "2026/09/03"]);
        assert_eq!(texts("sj"), ["19:06", "19:06:23", "19点06分"]);
        assert_eq!(texts("xq"), ["星期四", "周四"]);
        assert!(texts("rqi").is_empty());
    }

    #[test]
    fn expression_shortcuts() {
        assert_eq!(texts("v1+2"), ["3", "1+2=3"]);
        assert_eq!(
            texts("v123"),
            [
                "一百二十三",
                "壹佰贰拾叁",
                "一百二十三元整",
                "壹佰贰拾叁元整"
            ]
        );
        assert_eq!(
            texts("v123.5"),
            [
                "一百二十三点五",
                "壹佰贰拾叁点伍",
                "一百二十三元五角",
                "壹佰贰拾叁元伍角"
            ]
        );
        assert_eq!(texts("v123."), texts("v123"));
        // 超過角分的小數只有讀法；帶運算子的照常算
        assert_eq!(texts("v3.14159"), ["三点一四一五九", "叁点壹肆壹伍玖"]);
        assert_eq!(texts("v1.5*2"), ["3", "1.5*2=3"]);
        assert!(texts("v").is_empty());
        assert!(texts("v1+").is_empty());
        assert!(texts("very").is_empty());
    }

    #[test]
    fn expression_chars() {
        assert!(is_expression_char('7'));
        assert!(is_expression_char('('));
        assert!(!is_expression_char('='));
        assert!(!is_expression_char('x'));
    }
}
