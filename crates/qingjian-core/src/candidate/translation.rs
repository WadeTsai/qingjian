use serde::{Deserialize, Serialize};

use super::language::Language;
use super::sense::Sense;

/// 候選詞在單一學習語言下的翻譯。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Translation {
    /// 譯文所屬的學習語言。
    pub language: Language,

    /// 釋義，按常用程度排序，最多 [`Self::MAX_SENSES`] 條。
    senses: Vec<Sense>,
}

impl Translation {
    /// 釋義上限。超出的部分在構造時丟棄，保證候選框不會退化成迷你詞典。
    pub const MAX_SENSES: usize = 2;

    pub fn new(language: Language, mut senses: Vec<Sense>) -> Self {
        senses.truncate(Self::MAX_SENSES);
        Self { language, senses }
    }

    pub fn senses(&self) -> &[Sense] {
        &self.senses
    }

    pub fn senses_mut(&mut self) -> &mut [Sense] {
        &mut self.senses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::PartOfSpeech;

    #[test]
    fn keeps_at_most_two_senses() {
        let senses = ["develop", "development", "exploit"]
            .into_iter()
            .map(|text| Sense {
                part_of_speech: Some(PartOfSpeech::Verb),
                text: text.to_owned(),
                reading: None,
                fresh: false,
            })
            .collect();
        let translation = Translation::new(Language::English, senses);
        assert_eq!(translation.senses().len(), Translation::MAX_SENSES);
    }
}
