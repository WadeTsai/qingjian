//! 四則運算求值：`+ - * x / ^`、括號、小數、一元負號。遞迴下降，結果按整數或最多十位小數輸出。

/// 算式求值。算不了（語法錯、除零、溢位）返回 `None`。
pub fn evaluate(text: &str) -> Option<String> {
    let mut evaluator = Evaluator {
        bytes: text.as_bytes(),
        position: 0,
    };
    let value = evaluator.expression()?;
    if evaluator.position != evaluator.bytes.len() || !value.is_finite() {
        return None;
    }
    Some(format_number(value))
}

/// 遞迴下降求值器，只在 [`evaluate`] 裡用。
struct Evaluator<'a> {
    /// 算式文本。
    bytes: &'a [u8],

    /// 當前讀到的位置。
    position: usize,
}

impl Evaluator<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    /// 加減。
    fn expression(&mut self) -> Option<f64> {
        let mut value = self.term()?;
        while let Some(op @ (b'+' | b'-')) = self.peek() {
            self.position += 1;
            let rhs = self.term()?;
            value = if op == b'+' { value + rhs } else { value - rhs };
        }
        Some(value)
    }

    /// 乘除。`x` 也當乘號，鍵盤上打 `*` 要按 Shift。
    fn term(&mut self) -> Option<f64> {
        let mut value = self.power()?;
        while let Some(op @ (b'*' | b'x' | b'/')) = self.peek() {
            self.position += 1;
            let rhs = self.power()?;
            value = if op == b'/' {
                if rhs == 0.0 {
                    return None;
                }
                value / rhs
            } else {
                value * rhs
            };
        }
        Some(value)
    }

    /// 乘方，右結合。
    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        if self.peek() == Some(b'^') {
            self.position += 1;
            let exponent = self.power()?;
            return Some(base.powf(exponent));
        }
        Some(base)
    }

    /// 一元正負號。
    fn unary(&mut self) -> Option<f64> {
        match self.peek() {
            Some(b'-') => {
                self.position += 1;
                self.unary().map(|v| -v)
            }
            Some(b'+') => {
                self.position += 1;
                self.unary()
            }
            _ => self.atom(),
        }
    }

    /// 數字或括號。
    fn atom(&mut self) -> Option<f64> {
        if self.peek() == Some(b'(') {
            self.position += 1;
            let value = self.expression()?;
            if self.peek() != Some(b')') {
                return None;
            }
            self.position += 1;
            return Some(value);
        }
        let start = self.position;
        while matches!(self.peek(), Some(b'0'..=b'9' | b'.')) {
            self.position += 1;
        }
        if start == self.position {
            return None;
        }
        std::str::from_utf8(&self.bytes[start..self.position])
            .ok()?
            .parse()
            .ok()
    }
}

/// 整數不帶小數點，小數最多十位、去掉末尾的零。
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{value:.0}");
    }
    let text = format!("{value:.10}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        assert_eq!(evaluate("1+2").as_deref(), Some("3"));
        assert_eq!(evaluate("2x3+4").as_deref(), Some("10"));
        assert_eq!(evaluate("2*(3+4)").as_deref(), Some("14"));
        assert_eq!(evaluate("1/3").as_deref(), Some("0.3333333333"));
        assert_eq!(evaluate("2^10").as_deref(), Some("1024"));
        assert_eq!(evaluate("2^3^2").as_deref(), Some("512"));
        assert_eq!(evaluate("-3+5").as_deref(), Some("2"));
        assert_eq!(evaluate("0.1+0.2").as_deref(), Some("0.3"));
        assert_eq!(evaluate("1.5x2").as_deref(), Some("3"));
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(evaluate("1+"), None);
        assert_eq!(evaluate("1/0"), None);
        assert_eq!(evaluate("(1+2"), None);
        assert_eq!(evaluate("abc"), None);
        assert_eq!(evaluate(""), None);
    }
}
