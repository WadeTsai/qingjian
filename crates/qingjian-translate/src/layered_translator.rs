use qingjian_core::{Language, Translation, Translator};

use crate::{Glossary, PersonalGlossary};

/// 隨包釋義表 + 個人釋義表疊成一個譯者：個人表優先（使用者手改過的、雲端兜底寫的），查不到再查隨包表；
/// 釋義兜底學到的釋義寫進個人表，flush 時落盤。
pub struct LayeredTranslator {
    /// 個人釋義表。
    personal: PersonalGlossary,

    /// 隨包釋義表。
    bundled: Glossary,
}

impl LayeredTranslator {
    pub fn new(bundled: Glossary, personal: PersonalGlossary) -> Self {
        Self { personal, bundled }
    }

    /// 隨包釋義表的條數。
    pub fn len(&self) -> usize {
        self.bundled.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bundled.is_empty()
    }

    /// 個人釋義表的條數。
    pub fn personal_len(&self) -> usize {
        self.personal.len()
    }
}

impl Translator for LayeredTranslator {
    fn language(&self) -> Language {
        Translator::language(&self.bundled)
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        self.personal
            .translate(text)
            .or_else(|| self.bundled.translate(text))
    }

    fn learn(&mut self, word: &str, translation: Translation) {
        self.personal.insert(word, translation);
    }

    fn flush(&mut self) {
        if let Err(error) = self.personal.save() {
            tracing::warn!(%error, "个人释义表保存失败");
        }
    }
}
