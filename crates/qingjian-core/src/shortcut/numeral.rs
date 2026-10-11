//! 阿拉伯數字串轉中文數字：小寫（一百二十三）與大寫（壹佰貳拾叄，財務用），以及小數讀法與金額寫法。

/// 小寫數字與單位。
const LOWER: Numeral = Numeral {
    digits: ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"],
    units: ["", "十", "百", "千"],
    // 口語裡 10–19 開頭的「一」省掉：十五，而不是一十五
    drop_leading_one_ten: true,
};

/// 大寫數字與單位，財務票據寫法。
const UPPER: Numeral = Numeral {
    digits: ["零", "壹", "贰", "叁", "肆", "伍", "陆", "柒", "捌", "玖"],
    units: ["", "拾", "佰", "仟"],
    drop_leading_one_ten: false,
};

/// 億以內每四位一節的節單位；億以上把高位部分整體讀出再接「億」（一萬二千三百四十五億）。
const SECTIONS: [&str; 2] = ["", "万"];

/// 一「億」有幾位。
const YI_DIGITS: usize = 8;

/// 一套數字寫法。
struct Numeral {
    /// 0–9。
    digits: [&'static str; 10],

    /// 個、十、百、千。
    units: [&'static str; 4],

    /// 整個數以「一十」開頭時是否省掉「一」。
    drop_leading_one_ten: bool,
}

/// `123` → `一百二十三`。只接受純數字串；超過 16 位或空串原樣返回。
pub fn chinese_lower(digits: &str) -> String {
    LOWER.convert(digits)
}

/// `123` → `壹佰貳拾叄`。
pub fn chinese_upper(digits: &str) -> String {
    UPPER.convert(digits)
}

/// `123.5` → `一百二十三點五`：小數部分逐位讀。
pub fn chinese_decimal_lower(integer: &str, fraction: &str) -> String {
    LOWER.decimal(integer, fraction)
}

/// `123.5` → `壹佰貳拾叄點伍`。
pub fn chinese_decimal_upper(integer: &str, fraction: &str) -> String {
    UPPER.decimal(integer, fraction)
}

/// 金額小寫：`123.5` → `一百二十三元五角`，`123` → `一百二十三元整`。`fraction` 最多兩位（角、分）。
pub fn amount_lower(integer: &str, fraction: &str) -> String {
    LOWER.amount(integer, fraction)
}

/// 金額大寫：`123.05` → `壹佰貳拾叄元零伍分`。
pub fn amount_upper(integer: &str, fraction: &str) -> String {
    UPPER.amount(integer, fraction)
}

impl Numeral {
    fn decimal(&self, integer: &str, fraction: &str) -> String {
        let fraction: String = fraction
            .bytes()
            .map(|b| self.digits[usize::from(b - b'0')])
            .collect();
        format!("{}点{fraction}", self.convert(integer))
    }

    /// 票據寫法：元后無角分寫「整」；有分無角補「零」；不足一元不寫元。
    fn amount(&self, integer: &str, fraction: &str) -> String {
        let mut cents = fraction.bytes().map(|b| usize::from(b - b'0'));
        let jiao = cents.next().unwrap_or(0);
        let fen = cents.next().unwrap_or(0);
        let yuan = integer.trim_start_matches('0');
        let mut text = String::new();
        if !yuan.is_empty() || (jiao == 0 && fen == 0) {
            text.push_str(&self.convert(integer));
            text.push('元');
        }
        if jiao == 0 && fen == 0 {
            text.push('整');
            return text;
        }
        if jiao > 0 {
            text.push_str(self.digits[jiao]);
            text.push('角');
        } else if !yuan.is_empty() {
            text.push_str(self.digits[0]);
        }
        if fen > 0 {
            text.push_str(self.digits[fen]);
            text.push('分');
        }
        text
    }

    fn convert(&self, digits: &str) -> String {
        let digits = digits.trim_start_matches('0');
        if digits.is_empty() {
            return self.digits[0].to_owned();
        }
        if !digits.bytes().all(|b| b.is_ascii_digit()) || digits.len() > 16 {
            return digits.to_owned();
        }
        let text = self.below_yi_or_split(digits);
        if self.drop_leading_one_ten
            && let Some(rest) = text.strip_prefix(&format!("{}{}", self.digits[1], self.units[1]))
        {
            return format!("{}{rest}", self.units[1]);
        }
        text
    }

    /// 億以上：高位整體轉換後接「億」，低八位非零時接上（不足八位補「零」）。
    fn below_yi_or_split(&self, digits: &str) -> String {
        if digits.len() <= YI_DIGITS {
            return self.below_yi(digits);
        }
        let (high, low) = digits.split_at(digits.len() - YI_DIGITS);
        let mut text = self.below_yi_or_split(high);
        text.push('亿');
        let low = low.trim_start_matches('0');
        if !low.is_empty() {
            if low.len() < YI_DIGITS {
                text.push_str(self.digits[0]);
            }
            text.push_str(&self.below_yi(low));
        }
        text
    }

    /// 億以內（最多八位，無前導零）：從低位起每四位一節。
    fn below_yi(&self, digits: &str) -> String {
        let values: Vec<usize> = digits.bytes().map(|b| usize::from(b - b'0')).collect();
        let mut sections: Vec<&[usize]> = values.rchunks(4).collect();
        sections.reverse();
        let count = sections.len();
        let mut text = String::new();
        let mut pending_zero = false;
        for (index, section) in sections.iter().enumerate() {
            let value: usize = section.iter().fold(0, |acc, d| acc * 10 + d);
            if value == 0 {
                pending_zero = true;
                continue;
            }
            // 上一節是零、或本節不足四位（高位有零），補一個零
            if !text.is_empty() && (pending_zero || value < 1000) {
                text.push_str(self.digits[0]);
            }
            text.push_str(&self.section(section));
            text.push_str(SECTIONS[count - 1 - index]);
            pending_zero = false;
        }
        text
    }

    /// 一節（最多四位，非零）。中間的零合併成一個，末尾的零不寫。
    fn section(&self, digits: &[usize]) -> String {
        let mut text = String::new();
        let mut zero = false;
        for (index, &d) in digits.iter().enumerate() {
            let unit = self.units[digits.len() - 1 - index];
            if d == 0 {
                zero = true;
                continue;
            }
            if zero && !text.is_empty() {
                text.push_str(self.digits[0]);
            }
            zero = false;
            text.push_str(self.digits[d]);
            text.push_str(unit);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_numerals() {
        assert_eq!(chinese_lower("0"), "零");
        assert_eq!(chinese_lower("10"), "十");
        assert_eq!(chinese_lower("15"), "十五");
        assert_eq!(chinese_lower("105"), "一百零五");
        assert_eq!(chinese_lower("110"), "一百一十");
        assert_eq!(chinese_lower("1010"), "一千零一十");
        assert_eq!(chinese_lower("10000"), "一万");
        assert_eq!(chinese_lower("100001"), "十万零一");
        assert_eq!(chinese_lower("20003000"), "二千万三千");
        assert_eq!(chinese_lower("20000300"), "二千万零三百");
        assert_eq!(chinese_lower("100000001"), "一亿零一");
        assert_eq!(chinese_lower("1500000000"), "十五亿");
        assert_eq!(chinese_lower("120000300"), "一亿二千万零三百");
        assert_eq!(
            chinese_lower("1234567890123"),
            "一万二千三百四十五亿六千七百八十九万零一百二十三"
        );
    }

    #[test]
    fn decimals_read_digit_by_digit() {
        assert_eq!(chinese_decimal_lower("123", "5"), "一百二十三点五");
        assert_eq!(chinese_decimal_upper("123", "5"), "壹佰贰拾叁点伍");
        assert_eq!(chinese_decimal_lower("0", "05"), "零点零五");
        assert_eq!(chinese_decimal_lower("10", "250"), "十点二五零");
        assert_eq!(chinese_decimal_upper("10", "25"), "壹拾点贰伍");
    }

    #[test]
    fn amounts_follow_the_invoice_style() {
        assert_eq!(amount_lower("123", "5"), "一百二十三元五角");
        assert_eq!(amount_upper("123", "5"), "壹佰贰拾叁元伍角");
        assert_eq!(amount_lower("123", ""), "一百二十三元整");
        assert_eq!(amount_upper("123", "00"), "壹佰贰拾叁元整");
        assert_eq!(amount_upper("123", "45"), "壹佰贰拾叁元肆角伍分");
        assert_eq!(amount_upper("123", "05"), "壹佰贰拾叁元零伍分");
        assert_eq!(amount_upper("10", "5"), "壹拾元伍角");
        assert_eq!(amount_lower("10", "5"), "十元五角");
        // 不足一元不寫元；零就是零元整
        assert_eq!(amount_lower("0", "5"), "五角");
        assert_eq!(amount_upper("0", "05"), "伍分");
        assert_eq!(amount_lower("0", ""), "零元整");
        assert_eq!(amount_upper("100200", "3"), "壹拾万零贰佰元叁角");
    }

    #[test]
    fn upper_numerals_keep_leading_one() {
        assert_eq!(chinese_upper("15"), "壹拾伍");
        assert_eq!(chinese_upper("123"), "壹佰贰拾叁");
        assert_eq!(chinese_upper("10086"), "壹万零捌拾陆");
    }
}
