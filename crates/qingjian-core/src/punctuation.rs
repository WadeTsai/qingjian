//! 中文模式下的全形標點。
//!
//! 組句期間 `,` `.` 是翻頁鍵、`-` `=` 也是，這些由殼決定；這裡只回答「這個半形字元在中文模式下該變成什麼」。

/// 這個半形字元是不是會轉成全形標點的一員（`,` `.` `?` `!` 引號、括號這些）。
/// 殼用它分流組句中的標點：會轉的先把高亮候選上屏再補標點（`nihao,` 一氣打完「你好，」），
/// 其餘（`-` `/` `@`）仍進英文直輸段。數字後的 `.` 保持半形是 [`Punctuation::convert`] 的狀態決定的，
/// 這裡照算「會轉」，上屏候選後交給它判斷即可。
pub fn converts(c: char) -> bool {
    matches!(
        c,
        ',' | '.'
            | '?'
            | '!'
            | ':'
            | ';'
            | '('
            | ')'
            | '['
            | ']'
            | '<'
            | '>'
            | '\\'
            | '^'
            | '_'
            | '$'
            | '~'
            | '"'
            | '\''
    )
}

/// 引號成對切換的狀態。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Punctuation {
    /// 下一個 `"` 是左引號。
    double_quote_open: bool,

    /// 下一個 `'` 是左引號。
    single_quote_open: bool,

    /// 上一個上屏字元是 ASCII 數字：`3.14` 裡的 `.` 保持半形。
    after_digit: bool,
}

impl Punctuation {
    /// 半形字元對應的全形標點；不需要轉換的返回 `None`，殼把原字元交給應用。
    pub fn convert(&mut self, c: char) -> Option<&'static str> {
        let after_digit = std::mem::replace(&mut self.after_digit, false);
        let converted = match c {
            ',' => "，",
            '.' if after_digit => return None,
            '.' => "。",
            '?' => "？",
            '!' => "！",
            ':' => "：",
            ';' => "；",
            '(' => "（",
            ')' => "）",
            '[' => "【",
            ']' => "】",
            '<' => "《",
            '>' => "》",
            '\\' => "、",
            '^' => "……",
            '_' => "——",
            '$' => "￥",
            '~' => "～",
            '"' => {
                self.double_quote_open = !self.double_quote_open;
                if self.double_quote_open { "“" } else { "”" }
            }
            '\'' => {
                self.single_quote_open = !self.single_quote_open;
                if self.single_quote_open { "‘" } else { "’" }
            }
            _ => return None,
        };
        Some(converted)
    }

    /// 殼把沒有轉換的字元原樣交給應用後呼叫，用來記住「剛打了數字」。
    pub fn note_passthrough(&mut self, c: char) {
        self.after_digit = c.is_ascii_digit();
    }

    /// 有文本上屏後呼叫（候選、拼音、英文詞），數字狀態按最後一個字元更新。
    pub fn note_committed(&mut self, text: &str) {
        self.after_digit = text.chars().last().is_some_and(|c| c.is_ascii_digit());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_common_marks_and_toggles_quotes() {
        let mut p = Punctuation::default();
        assert_eq!(p.convert(','), Some("，"));
        assert_eq!(p.convert('"'), Some("“"));
        assert_eq!(p.convert('"'), Some("”"));
        assert_eq!(p.convert('\''), Some("‘"));
        assert_eq!(p.convert('\''), Some("’"));
        assert_eq!(p.convert('a'), None);
        assert_eq!(p.convert('-'), None);
    }

    #[test]
    fn period_after_digit_stays_ascii() {
        let mut p = Punctuation::default();
        p.note_passthrough('3');
        assert_eq!(p.convert('.'), None);
        assert_eq!(p.convert('.'), Some("。"));
        p.note_committed("第1");
        assert_eq!(p.convert('.'), None);
        p.note_committed("开发");
        assert_eq!(p.convert('.'), Some("。"));
    }

    /// `converts` 與 `convert` 能轉的字元集合一致（`after_digit` 的 `.` 除外，那是狀態決定的）。
    #[test]
    fn converts_covers_exactly_the_convertible_characters() {
        for c in ('\t'..='~').filter(char::is_ascii_graphic) {
            let convertible = Punctuation::default().convert(c).is_some();
            assert_eq!(converts(c), convertible, "{c}");
        }
        assert!(converts('\''));
        assert!(!converts('a'));
        assert!(!converts('-'));
        assert!(!converts('/'));
        assert!(!converts('@'));
        assert!(!converts('中'));
    }
}
