/// 把鍵盤輸入常用的 `lue` / `nue` 轉成詞庫規範形式。
///
/// 詞庫用 `v` 表示 ü。j / q / x / y 後 ü 的正字法寫作 `u`，但鍵盤上 `jv` / `qv` / `xv` / `yv`
/// 是常見輸入習慣（與微軟、搜狗一致），也歸一到規範寫法。其他音節原樣返回。
pub fn canonical_syllable(text: &str) -> &str {
    match text {
        "lue" => "lve",
        "nue" => "nve",
        "jv" => "ju",
        "qv" => "qu",
        "xv" => "xu",
        "yv" => "yu",
        _ => text,
    }
}

/// 一個音節的查詢條件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyllablePattern<'a> {
    /// 使用者敲的字母：完整音節，或音節字首 / 聲母。
    pub text: &'a str,

    /// 為真時要求音節與 `text` 完全相等，否則只要求以 `text` 開頭（簡拼、未打完的音節）。
    pub complete: bool,
}

impl<'a> SyllablePattern<'a> {
    pub fn complete(text: &'a str) -> Self {
        Self {
            text,
            complete: true,
        }
    }

    pub fn prefix(text: &'a str) -> Self {
        Self {
            text,
            complete: false,
        }
    }

    pub fn accepts(&self, syllable: &str) -> bool {
        let text = canonical_syllable(self.text);
        let syllable = canonical_syllable(syllable);
        if self.complete {
            syllable == text
        } else {
            syllable.starts_with(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_keyboard_spelling_for_u_umlaut() {
        assert_eq!(canonical_syllable("lue"), "lve");
        assert_eq!(canonical_syllable("nue"), "nve");
        assert_eq!(canonical_syllable("lve"), "lve");
        assert_eq!(canonical_syllable("nve"), "nve");
        assert_eq!(canonical_syllable("xue"), "xue");
        assert_eq!(canonical_syllable("jv"), "ju");
        assert_eq!(canonical_syllable("qv"), "qu");
        assert_eq!(canonical_syllable("xv"), "xu");
        assert_eq!(canonical_syllable("yv"), "yu");
    }

    #[test]
    fn accepts_keyboard_and_canonical_spellings_equally() {
        assert!(SyllablePattern::complete("lue").accepts("lve"));
        assert!(SyllablePattern::complete("nve").accepts("nue"));
        assert!(!SyllablePattern::complete("xue").accepts("xve"));
    }
}
