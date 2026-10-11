use std::collections::HashMap;
use std::path::Path;

/// 詞 → emoji 列表。表裡一個詞後面的 emoji 按相關度排好（名字正好是這個詞的在前）。
#[derive(Debug, Default)]
pub struct EmojiTable {
    /// 詞 → emoji。
    entries: HashMap<String, Vec<String>>,
}

impl EmojiTable {
    /// 解析 TSV：`詞\temoji emoji …`，空行與 `#` 開頭跳過；格式不對的行返回行號。
    pub fn parse(source: &str) -> Result<Self, usize> {
        let mut entries: HashMap<String, Vec<String>> = HashMap::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (word, emojis) = line.split_once('\t').ok_or(index + 1)?;
            let list = entries.entry(word.to_owned()).or_default();
            for emoji in emojis.split(' ').filter(|e| !e.is_empty()) {
                if !list.iter().any(|e| e == emoji) {
                    list.push(emoji.to_owned());
                }
            }
        }
        Ok(Self { entries })
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let source = std::fs::read_to_string(path)?;
        Self::parse(&source).map_err(|line| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("emoji table line {line}: malformed entry"),
            )
        })
    }

    /// 這個詞對應的 emoji（沒有就是空）。
    /// 併入另一張表（中文表 + 英文表合成一張：鍵是不同文字，不會撞）；同鍵的 emoji 追加去重。
    pub fn merge(&mut self, other: EmojiTable) {
        for (word, emojis) in other.entries {
            let list = self.entries.entry(word).or_default();
            for emoji in emojis {
                if !list.contains(&emoji) {
                    list.push(emoji);
                }
            }
        }
    }

    pub fn lookup(&self, word: &str) -> &[String] {
        self.entries.get(word).map_or(&[], Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_deduplicates() {
        let table = EmojiTable::parse("# 注释\n笑\t😄 😆\n笑\t😆 🤣\n").unwrap();
        assert_eq!(table.lookup("笑"), ["😄", "😆", "🤣"]);
        assert!(table.lookup("哭").is_empty());
        assert_eq!(EmojiTable::parse("坏行\n").unwrap_err(), 1);
    }

    #[test]
    fn merge_appends_without_duplicates() {
        let mut zh = EmojiTable::parse("笑\t😄 😀\n").unwrap();
        let en = EmojiTable::parse("smile\t😀\n笑\t😀 😁\n").unwrap();
        zh.merge(en);
        assert_eq!(zh.lookup("smile"), ["😀"]);
        assert_eq!(zh.lookup("笑"), ["😄", "😀", "😁"]);
        assert_eq!(zh.len(), 2);
    }
}
