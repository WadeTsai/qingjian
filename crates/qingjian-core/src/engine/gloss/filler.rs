use super::FilledGloss;
use crate::candidate::Language;

/// 釋義兜底的提供方。實現可以聯網，介面非阻塞：`request` 只入隊，`poll` 只取已到的。
pub trait GlossFiller: Send {
    /// 是否真的會去問；`false` 時 Engine 不入隊。
    fn is_enabled(&self) -> bool {
        true
    }

    /// 請給 `word` 寫一條 `language` 的釋義。實現自行去重、攢批，問過的不再問。
    fn request(&mut self, language: Language, word: &str);

    /// 取已到達的釋義，沒有返回空。
    fn poll(&mut self) -> Vec<FilledGloss>;
}

/// 不兜底。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoGlossFiller;

impl GlossFiller for NoGlossFiller {
    fn is_enabled(&self) -> bool {
        false
    }

    fn request(&mut self, _language: Language, _word: &str) {}

    fn poll(&mut self) -> Vec<FilledGloss> {
        Vec::new()
    }
}
