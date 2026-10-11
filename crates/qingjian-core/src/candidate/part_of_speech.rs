use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 詞性。縮寫沿用英文詞典慣例，各平臺按學習語言決定顯示文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartOfSpeech {
    /// 名詞 n.
    Noun,

    /// 動詞 v.
    Verb,

    /// 形容詞 adj.
    Adjective,

    /// 副詞 adv.
    Adverb,

    /// 代詞 pron.
    Pronoun,

    /// 介詞 prep.
    Preposition,

    /// 連詞 conj.
    Conjunction,

    /// 數詞 num.
    Numeral,

    /// 量詞 m.
    Measure,

    /// 助詞 part.
    Particle,

    /// 嘆詞 int.
    Interjection,

    /// 短語 phr.
    Phrase,
}

impl PartOfSpeech {
    pub fn abbreviation(self) -> &'static str {
        match self {
            Self::Noun => "n.",
            Self::Verb => "v.",
            Self::Adjective => "adj.",
            Self::Adverb => "adv.",
            Self::Pronoun => "pron.",
            Self::Preposition => "prep.",
            Self::Conjunction => "conj.",
            Self::Numeral => "num.",
            Self::Measure => "m.",
            Self::Particle => "part.",
            Self::Interjection => "int.",
            Self::Phrase => "phr.",
        }
    }
}

impl fmt::Display for PartOfSpeech {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.abbreviation())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown part of speech: {0}")]
pub struct UnknownPartOfSpeech(pub String);

impl FromStr for PartOfSpeech {
    type Err = UnknownPartOfSpeech;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().trim_end_matches('.').to_ascii_lowercase().as_str() {
            "n" | "noun" => Ok(Self::Noun),
            "v" | "verb" => Ok(Self::Verb),
            "adj" => Ok(Self::Adjective),
            "adv" => Ok(Self::Adverb),
            "pron" => Ok(Self::Pronoun),
            "prep" => Ok(Self::Preposition),
            "conj" => Ok(Self::Conjunction),
            "num" => Ok(Self::Numeral),
            "m" | "mw" => Ok(Self::Measure),
            "part" => Ok(Self::Particle),
            "int" | "interj" => Ok(Self::Interjection),
            "phr" | "phrase" => Ok(Self::Phrase),
            other => Err(UnknownPartOfSpeech(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_or_without_dot() {
        assert_eq!(
            "adj.".parse::<PartOfSpeech>().unwrap(),
            PartOfSpeech::Adjective
        );
        assert_eq!("N".parse::<PartOfSpeech>().unwrap(), PartOfSpeech::Noun);
        assert!("xyz".parse::<PartOfSpeech>().is_err());
    }
}
