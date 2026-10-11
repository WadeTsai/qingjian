//! 日誌裡的金鑰掩碼：程序載入到金鑰就登記一次，日誌落盤前把登記過的值換成 `***`。
//! 逐處「別把金鑰記進日誌」防不住第三方庫的報錯原文（dotenvy 帶出過整行 `.env`），所以在寫入口統一兜底。

mod masking_writer;

use std::borrow::Cow;
use std::sync::RwLock;

pub use self::masking_writer::MaskingWriter;

/// 比這短的值不登記：太短的串會誤傷正常日誌，也不是真金鑰。
const MIN_SECRET_LEN: usize = 8;

const MASK: &str = "***";

static SECRETS: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// 登記一個金鑰；重複登記、過短的值忽略。
pub fn register(secret: &str) {
    let secret = secret.trim();
    if secret.len() < MIN_SECRET_LEN {
        return;
    }
    let Ok(mut secrets) = SECRETS.write() else {
        return;
    };
    if !secrets.iter().any(|known| known == secret) {
        secrets.push(secret.to_owned());
    }
}

/// 把文本里登記過的金鑰換成 `***`；沒有命中就原樣借回去。
pub fn mask(text: &str) -> Cow<'_, str> {
    let Ok(secrets) = SECRETS.read() else {
        return Cow::Borrowed(text);
    };
    let mut masked = Cow::Borrowed(text);
    for secret in secrets.iter() {
        if masked.contains(secret.as_str()) {
            masked = Cow::Owned(masked.replace(secret.as_str(), MASK));
        }
    }
    masked
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn registered_secrets_are_masked_everywhere_they_appear() {
        register("sk-test-0123456789abcdef");
        register("short");
        let line =
            "error parsing line: 'KEY=sk-test-0123456789abcdef' and again sk-test-0123456789abcdef";
        assert_eq!(mask(line), "error parsing line: 'KEY=***' and again ***");
        assert_eq!(mask("short stays"), "short stays");
        assert!(matches!(mask("nothing here"), Cow::Borrowed(_)));
    }

    #[test]
    fn writer_masks_before_the_bytes_reach_the_file() {
        register("sk-writer-0123456789abcdef");
        let mut writer = MaskingWriter::new(Vec::new());
        writer
            .write_all("WARN 重载失败 line=sk-writer-0123456789abcdef\n".as_bytes())
            .unwrap();
        assert_eq!(
            String::from_utf8(writer.into_inner()).unwrap(),
            "WARN 重载失败 line=***\n"
        );
    }
}
