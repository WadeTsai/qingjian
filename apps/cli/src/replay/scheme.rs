//! 回放期間按日誌裡記的方案裝配引擎。
//!
//! 每條上屏的 `scheme` 欄位寫著當時用的是哪套方案（雙拼的鍵、`zhuyin`、形碼的 `wubi`、全拼為空），
//! 回放要照著還原，否則拿拼音的讀法去喂形碼的鍵、或者反過來，算出來的命中率沒有意義。
//! 整份日誌通常只有一套方案，所以只在方案串變化時才動碼錶——`Engine::set_code_table` 收所有權，
//! 換一次要克隆一份 8.9 萬條的碼錶。

use qingjian_core::Engine;
use qingjian_dictionary::CodeTable;
use qingjian_platform::Scheme;

/// 按日誌裡的方案串裝配引擎，並記住當前裝的是哪套。
pub struct SchemeSwitcher {
    /// 當前已裝配的方案串；沒裝過是 `None`。
    current: Option<String>,

    /// 日誌裡出現形碼時用的碼錶；沒給 `--wubi` 就是 `None`，那部分日誌回放不了。
    table: Option<CodeTable>,
}

impl SchemeSwitcher {
    pub fn new(table: Option<CodeTable>) -> Self {
        Self {
            current: None,
            table,
        }
    }

    /// 這條日誌能不能回放：帶形碼的要有碼錶，別的方案都行。
    pub fn can_replay(&self, key: &str) -> bool {
        !split(key).1 || self.table.is_some()
    }

    /// 按 `key` 裝配引擎。雙拼與注音的開關每次都設（便宜）；碼錶只在方案串變了時換。
    pub fn apply(&mut self, engine: &mut Engine, key: &str) {
        let (pinyin, wubi) = split(key);
        engine.set_shuangpin(pinyin.shuangpin());
        engine.set_zhuyin_mode(pinyin == Scheme::Zhuyin);
        engine.set_phonetic(pinyin.is_on());
        if self.current.as_deref() == Some(key) {
            return;
        }
        self.current = Some(key.to_owned());
        engine.set_code_table(if wubi { self.table.clone() } else { None });
    }
}

/// 日誌裡的方案串拆成「拼音側方案 + 形碼開沒開」：
/// `"xiaohe+wubi"` → 小鶴 + 開；`"wubi"` → 拼音關 + 開；`""` → 全拼 + 關。
/// 空串與認不出來的寫法都按全拼（老日誌裡沒有這個欄位）。
fn split(key: &str) -> (Scheme, bool) {
    if let Some(pinyin) = key.strip_suffix("+wubi") {
        return (pinyin.parse().unwrap_or_default(), true);
    }
    if key == "wubi" {
        return (Scheme::Off, true);
    }
    (key.parse().unwrap_or_default(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_which_schemes_need_a_table() {
        let without = SchemeSwitcher::new(None);
        assert!(without.can_replay(""));
        assert!(without.can_replay("xiaohe"));
        assert!(without.can_replay("zhuyin"));
        assert!(!without.can_replay("wubi"));

        let table = CodeTable::parse("一\tggll\t100\n").unwrap();
        assert!(SchemeSwitcher::new(Some(table)).can_replay("wubi"));
    }

    #[test]
    fn applying_a_code_scheme_hands_the_table_to_the_engine() {
        use qingjian_core::Engine;
        use qingjian_dictionary::Dictionary;

        let table = CodeTable::parse("一\tggll\t100\n").unwrap();
        let mut switcher = SchemeSwitcher::new(Some(table));
        let mut engine = Engine::new(Dictionary::parse("开\tkai\t100\n").unwrap());

        switcher.apply(&mut engine, "wubi");
        engine.set_input("ggll");
        assert_eq!(engine.query().unwrap().candidates.items[0].text, "一");

        // 切回全拼：碼錶卸掉，同一串編碼不再出中文候選
        switcher.apply(&mut engine, "");
        engine.set_input("ggll");
        let texts: Vec<String> = engine
            .query()
            .map(|q| q.candidates.items.into_iter().map(|c| c.text).collect())
            .unwrap_or_default();
        assert!(!texts.contains(&"一".to_owned()));
    }

    #[test]
    fn scheme_strings_split_into_the_two_axes() {
        // 老日誌沒有 scheme 欄位，解出來是全拼、不開形碼
        assert_eq!(split(""), (Scheme::Pinyin, false));
        assert_eq!(split("没见过的写法"), (Scheme::Pinyin, false));
        assert_eq!(
            split("xiaohe"),
            (
                Scheme::Shuangpin(qingjian_core::ShuangpinScheme::Xiaohe),
                false
            )
        );
        // 只用形碼
        assert_eq!(split("wubi"), (Scheme::Off, true));
        // 混輸：拼音側那部分照解，形碼跟著開
        assert_eq!(
            split("xiaohe+wubi"),
            (
                Scheme::Shuangpin(qingjian_core::ShuangpinScheme::Xiaohe),
                true
            )
        );
        assert_eq!(split("pinyin+wubi"), (Scheme::Pinyin, true));
        assert_eq!(split("zhuyin+wubi"), (Scheme::Zhuyin, true));
    }
}
