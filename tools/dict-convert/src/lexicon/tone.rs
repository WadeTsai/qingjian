//! 拼音寫法歸一：帶聲調符號的（Unihan 的 `xíng`）與帶聲調數字的（通用詞表的 `wei4'shen2'me`）都轉成
//! 不帶聲調、ü 寫 v 的小寫音節。

use qingjian_dictionary::canonical_syllable;

/// `xíng` → `xing`，`lǜ` → `lv`，`ê̄` 這類罕見記號轉成對應字母；組合用變音符號（U+0300–U+036F）丟掉。
pub fn strip_tone(reading: &str) -> String {
    let mut out = String::with_capacity(reading.len());
    for c in reading.chars() {
        let plain = match c {
            'ā' | 'á' | 'ǎ' | 'à' => 'a',
            'ē' | 'é' | 'ě' | 'è' | 'ê' | 'ế' | 'ề' => 'e',
            'ī' | 'í' | 'ǐ' | 'ì' => 'i',
            'ō' | 'ó' | 'ǒ' | 'ò' => 'o',
            'ū' | 'ú' | 'ǔ' | 'ù' => 'u',
            'ü' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' => 'v',
            'ḿ' => 'm',
            'ń' | 'ň' | 'ǹ' => 'n',
            '\u{0300}'..='\u{036f}' => continue,
            c if c.is_ascii_digit() => continue,
            c => c.to_ascii_lowercase(),
        };
        out.push(plain);
    }
    canonical_syllable(&out).to_owned()
}

/// 通用詞表的 `wei4'shen2'me` → `["wei", "shen", "me"]`；`lv4` 保持 v。空段跳過。
pub fn numeric_syllables(pinyin: &str) -> Vec<String> {
    pinyin
        .split('\'')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(strip_tone)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tone_marks_and_digits() {
        assert_eq!(strip_tone("xíng"), "xing");
        assert_eq!(strip_tone("lǜ"), "lv");
        assert_eq!(strip_tone("nǚ"), "nv");
        assert_eq!(strip_tone("lüè"), "lve");
        assert_eq!(strip_tone("nüè"), "nve");
        assert_eq!(strip_tone("lue4"), "lve");
        assert_eq!(strip_tone("nue4"), "nve");
        assert_eq!(strip_tone("shi4"), "shi");
        assert_eq!(strip_tone("de"), "de");
    }

    #[test]
    fn splits_numeric_pinyin() {
        assert_eq!(numeric_syllables("wei4'shen2'me"), ["wei", "shen", "me"]);
        assert_eq!(numeric_syllables("nv3'ren2"), ["nv", "ren"]);
        assert_eq!(numeric_syllables("zhe4'er"), ["zhe", "er"]);
    }
}
