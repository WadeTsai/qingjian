//! 一個詞的生成結果，JSONL 裡一行一個。

mod english_gloss_entry;
mod gloss_entry;
mod japanese_sense;
mod pinyin_entry;

pub use english_gloss_entry::EnglishGlossEntry;
pub use gloss_entry::GlossEntry;
pub use japanese_sense::JapaneseSense;
pub use pinyin_entry::PinyinEntry;
