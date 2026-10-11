//! emoji 候選：按候選詞查 emoji（笑 → 😄），緊跟在對應詞後面。資料表由 Unicode CLDR 的中文 annotations 生成
//! （`assets/emoji/emoji-zh.tsv`，`詞\temoji emoji …`），`dict-convert emoji` 轉換。

mod table;

pub use table::EmojiTable;
