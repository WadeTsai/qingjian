use crate::candidate::{Language, Translation};

/// 候選翻譯 annotation 的提供方。實現必須是純查表級別的開銷，不能做網路請求。
pub trait Translator: Send {
    /// 當前學習語言。
    fn language(&self) -> Language;

    /// 查不到返回 `None`，候選框那一行留空。
    fn translate(&self, text: &str) -> Option<Translation>;

    /// 釋義兜底從雲端寫好了一條釋義：記進個人釋義表，下次 [`Self::translate`] 查得到。只查隨包表的實現留空即可。
    fn learn(&mut self, _word: &str, _translation: Translation) {}

    /// 把個人釋義表落盤；失敗只記日誌。
    fn flush(&mut self) {}
}

/// 不做翻譯。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoTranslator;

impl Translator for NoTranslator {
    fn language(&self) -> Language {
        Language::Chinese
    }

    fn translate(&self, _text: &str) -> Option<Translation> {
        None
    }
}
