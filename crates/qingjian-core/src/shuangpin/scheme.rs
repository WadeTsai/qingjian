use std::fmt;
use std::str::FromStr;

use qingjian_dictionary::canonical_syllable;
use serde::{Deserialize, Serialize};

use super::table::Table;
use super::{Decoded, Unit, table};
use crate::parser;

/// 雙拼方案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    /// 小鶴雙拼。
    Xiaohe,

    /// 自然碼。
    Ziranma,

    /// 微軟雙拼。
    Microsoft,

    /// 搜狗雙拼。
    Sogou,

    /// 智慧 ABC。
    Abc,

    /// 小浪雙拼。
    Xiaolang,

    /// 首道雙拼。
    Shoudao,
}

impl Scheme {
    /// 全部方案，設定介面按這個順序列出。
    pub const ALL: [Self; 7] = [
        Self::Xiaohe,
        Self::Ziranma,
        Self::Microsoft,
        Self::Sogou,
        Self::Abc,
        Self::Xiaolang,
        Self::Shoudao,
    ];

    /// 配置檔案裡的寫法。`const`：設定介面要按方案列表建常量表（見 `qingjian_platform::Scheme`）。
    pub const fn key(self) -> &'static str {
        match self {
            Self::Xiaohe => "xiaohe",
            Self::Ziranma => "ziranma",
            Self::Microsoft => "microsoft",
            Self::Sogou => "sogou",
            Self::Abc => "abc",
            Self::Xiaolang => "xiaolang",
            Self::Shoudao => "shoudao",
        }
    }

    /// 介面上的名字。`const` 的理由同上。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Xiaohe => "小鶴雙拼",
            Self::Ziranma => "自然碼",
            Self::Microsoft => "微軟雙拼",
            Self::Sogou => "搜狗雙拼",
            Self::Abc => "智能ABC",
            Self::Xiaolang => "小浪雙拼",
            Self::Shoudao => "首道雙拼",
        }
    }

    fn table(self) -> &'static Table {
        match self {
            Self::Xiaohe => &table::XIAOHE,
            Self::Ziranma => &table::ZIRANMA,
            Self::Microsoft => &table::MICROSOFT,
            Self::Sogou => &table::SOGOU,
            Self::Abc => &table::ABC,
            Self::Xiaolang => &table::XIAOLANG,
            Self::Shoudao => &table::SHOUDAO,
        }
    }

    /// 這套方案是否用到 `;` 鍵。
    pub fn uses_semicolon(self) -> bool {
        self.table().semicolon
    }

    /// `c` 是不是這套方案的鍵：小寫字母，微軟 / 搜狗再加 `;`。
    pub fn is_key(self, c: char) -> bool {
        c.is_ascii_lowercase() || (c == ';' && self.uses_semicolon())
    }

    /// 鍵 `key` 當聲母時是什麼：翹舌聲母按方案對映，其他子音（含 y w）是自己，母音鍵與 `;` 不是聲母。
    pub fn initial(self, key: char) -> Option<&'static str> {
        if let Some((_, initial)) = self
            .table()
            .digraph_initials
            .iter()
            .find(|(k, _)| *k == key)
        {
            return Some(initial);
        }
        parser::INITIALS
            .iter()
            .copied()
            .find(|initial| initial.len() == 1 && initial.starts_with(key))
    }

    /// 鍵 `key` 當韻母時的候選韻母（按優先順序）。
    pub fn finals(self, key: char) -> &'static [&'static str] {
        self.table()
            .finals
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(&[], |(_, finals)| finals)
    }

    /// 兩個鍵拼成的音節；拼不出合法音節時為 `None`。
    pub fn syllable(self, first: char, second: char) -> Option<String> {
        let pair = [first, second];
        for (syllable, spellings) in self.table().zero_initials {
            if spellings.iter().any(|s| s.chars().eq(pair.iter().copied())) {
                return Some((*syllable).to_owned());
            }
        }
        let initial = self.initial(first)?;
        self.finals(second)
            .iter()
            .find_map(|final_| {
                // jv / qv 這類全拼鍵盤別名也是合法音節，解碼結果統一成詞庫寫法
                let syllable = format!("{initial}{final_}");
                parser::is_syllable(&syllable).then(|| canonical_syllable(&syllable).to_owned())
            })
            .or_else(|| {
                // ü 在 j / q / x / y 後寫作 u；同鍵已有的合法音節仍優先。
                (matches!(initial, "j" | "q" | "x" | "y") && self.finals(second).contains(&"v"))
                    .then(|| format!("{initial}u"))
            })
    }

    /// 一個全拼音節的主寫法（兩個鍵）。測試與文件用；拆成聲母 + 韻母后查表，零聲母查零聲母表。
    pub fn encode(self, syllable: &str) -> Option<[char; 2]> {
        let table = self.table();
        if let Some((_, spellings)) = table.zero_initials.iter().find(|(s, _)| *s == syllable) {
            let mut chars = spellings[0].chars();
            return Some([chars.next()?, chars.next()?]);
        }
        let initial = parser::INITIALS
            .iter()
            .filter(|initial| syllable.starts_with(*initial))
            .max_by_key(|initial| initial.len())?;
        let final_ = &syllable[initial.len()..];
        let first = table
            .digraph_initials
            .iter()
            .find(|(_, i)| i == initial)
            .map(|(key, _)| *key)
            .or_else(|| initial.chars().next())?;
        let second = table
            .finals
            .iter()
            .find(|(_, finals)| finals.contains(&final_))
            .map(|(key, _)| *key)?;
        Some([first, second])
    }

    /// 把敲的鍵翻成全拼。兩鍵一組從左到右配對；配不出合法音節的位置起全部原樣留作尾巴；
    /// 末尾落單的一鍵當聲母（或母音）字首；使用者自己敲的 `'` 結束當前配對。
    pub fn decode(self, keys: &str) -> Decoded {
        let chars: Vec<char> = keys.chars().collect();
        let mut units = Vec::with_capacity(chars.len() / 2 + 1);
        let mut index = 0;
        while index < chars.len() {
            let first = chars[index];
            if first == '\'' {
                units.push(Unit::separator());
                index += 1;
                continue;
            }
            let second = chars.get(index + 1).copied().filter(|c| *c != '\'');
            let unit = match second {
                Some(second) => self.syllable(first, second).map(|pinyin| Unit {
                    keys: [first, second].iter().collect(),
                    pinyin,
                    complete: true,
                }),
                None => self.partial(first).map(|pinyin| Unit {
                    keys: first.to_string(),
                    pinyin,
                    complete: false,
                }),
            };
            let Some(unit) = unit else {
                break;
            };
            index += unit.keys.len();
            units.push(unit);
        }
        Decoded::new(units, chars[index..].iter().collect())
    }

    /// 落單的一鍵代表的字首：聲母鍵是聲母，母音鍵是母音本身（`a` 後面可能是 ai / an / ang / ao）。
    fn partial(self, key: char) -> Option<String> {
        if let Some(initial) = self.initial(key) {
            return Some(initial.to_owned());
        }
        match self {
            // 小浪與首道的 `e` 鍵是翹舌聲母，零聲母 e 一族改用 `u` 引導
            Self::Xiaolang | Self::Shoudao => match key {
                'a' => Some("a".to_owned()),
                'o' => Some("o".to_owned()),
                'u' => Some("e".to_owned()),
                _ => None,
            },
            _ => matches!(key, 'a' | 'e' | 'o').then(|| key.to_string()),
        }
    }
}

impl FromStr for Scheme {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|scheme| scheme.key() == text.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("unknown shuangpin scheme: {text:?}"))
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每套方案：全部音節都能編成兩鍵、再解回來是同一個音節。
    /// 只有 `lue` / `nue`（與 `lve` / `nve` 同鍵）和 `lo`（與 `luo` 同鍵）解回來是另一個寫法。
    #[test]
    fn every_syllable_round_trips() {
        for scheme in Scheme::ALL {
            for syllable in parser::SYLLABLES {
                let keys = scheme
                    .encode(syllable)
                    .unwrap_or_else(|| panic!("{scheme}: {syllable} 編不出"));
                let decoded = scheme
                    .syllable(keys[0], keys[1])
                    .unwrap_or_else(|| panic!("{scheme}: {syllable} → {keys:?} 解不回"));
                let expected = match *syllable {
                    // 首道的 ue 與 üe 分在兩個鍵：lue / nue 按 ue 鍵編出來撞上 lai / nai
                    "lue" if scheme == Scheme::Shoudao => "lai",
                    "nue" if scheme == Scheme::Shoudao => "nai",
                    // 方案自帶的同碼：`dk` 是 den / dia，取鍵點陣圖上排在前面的 en
                    "dia" if scheme == Scheme::Shoudao => "den",
                    "lue" => "lve",
                    "nue" => "nve",
                    "lo" => "luo",
                    // 全拼的鍵盤別名，雙拼解成詞庫寫法
                    "jv" => "ju",
                    "qv" => "qu",
                    "xv" => "xu",
                    "yv" => "yu",
                    "eng" if scheme == Scheme::Xiaolang => "en",
                    "dia" if scheme == Scheme::Xiaolang => "dai",
                    "lia" if scheme == Scheme::Xiaolang => "lai",
                    "nen" if scheme == Scheme::Xiaolang => "niang",
                    other => other,
                };
                assert_eq!(decoded, expected, "{scheme}: {syllable} → {keys:?}");
            }
        }
    }

    /// 兩鍵組合最多對應一個音節：解碼不會有歧義。
    #[test]
    fn every_key_pair_is_unambiguous() {
        for scheme in Scheme::ALL {
            let keys: Vec<char> = "abcdefghijklmnopqrstuvwxyz;".chars().collect();
            for first in &keys {
                for second in &keys {
                    // `syllable` 本身就是確定的；這裡確認零聲母寫法與「聲母 + 韻母」不撞車
                    let zero = scheme
                        .table()
                        .zero_initials
                        .iter()
                        .filter(|(_, spellings)| {
                            spellings.iter().any(|s| s.chars().eq([*first, *second]))
                        })
                        .count();
                    let max_zero = if scheme == Scheme::Xiaolang && *first == 'u' && *second == 'n'
                    {
                        2
                    } else {
                        1
                    };
                    assert!(
                        zero <= max_zero,
                        "{scheme}: {first}{second} 對應多個零聲母音節"
                    );
                    if zero >= 1 {
                        // 首道的 `en` `er` 以 sh 鍵開頭，但 sh 配不出 ian / ie，仍然不撞
                        let clash = scheme.initial(*first).is_some_and(|initial| {
                            scheme
                                .finals(*second)
                                .iter()
                                .any(|final_| parser::is_syllable(&format!("{initial}{final_}")))
                        });
                        assert!(
                            !clash,
                            "{scheme}: {first}{second} 既是零聲母寫法又能拼成聲母 + 韻母"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn well_known_words_decode_per_scheme() {
        let cases = [
            (Scheme::Xiaohe, "nihc", "ni'hao"),
            (Scheme::Xiaohe, "vsgo", "zhong'guo"),
            (Scheme::Xiaohe, "ulpb", "shuang'pin"),
            (Scheme::Xiaohe, "xtxi", "xue'xi"),
            (Scheme::Xiaohe, "uiui", "shi'shi"),
            (Scheme::Xiaohe, "wdgo", "wai'guo"),
            (Scheme::Xiaohe, "yske", "yong'ke"),
            (Scheme::Xiaohe, "nvhd", "nv'hai"),
            (Scheme::Xiaohe, "ahnv", "ang'nv"),
            (Scheme::Ziranma, "nihk", "ni'hao"),
            (Scheme::Ziranma, "udpn", "shuang'pin"),
            (Scheme::Ziranma, "xtxi", "xue'xi"),
            (Scheme::Ziranma, "eeyu", "e'yu"),
            (Scheme::Microsoft, "nihk", "ni'hao"),
            (Scheme::Microsoft, "udpn", "shuang'pin"),
            (Scheme::Microsoft, "ly", "lv"),
            (Scheme::Microsoft, "oaol", "a'ai"),
            (Scheme::Microsoft, "lv", "lve"),
            (Scheme::Microsoft, "x;", "xing"),
            (Scheme::Sogou, "nihk", "ni'hao"),
            (Scheme::Sogou, "oe", "e"),
            (Scheme::Sogou, "y;", "ying"),
            (Scheme::Abc, "nihk", "ni'hao"),
            (Scheme::Abc, "asgo", "zhong'guo"),
            (Scheme::Abc, "vtpc", "shuang'pin"),
            (Scheme::Abc, "xmxi", "xue'xi"),
            (Scheme::Abc, "vivi", "shi'shi"),
            (Scheme::Abc, "wlgo", "wai'guo"),
            (Scheme::Abc, "orqx", "er'qie"),
            (Scheme::Abc, "nvhl", "nv'hai"),
            (Scheme::Abc, "ohnv", "ang'nv"),
            (Scheme::Shoudao, "nihd", "ni'hao"),
            (Scheme::Shoudao, "vhgo", "zhong'guo"),
            (Scheme::Shoudao, "expc", "shuang'pin"),
            (Scheme::Shoudao, "xlxi", "xue'xi"),
            (Scheme::Shoudao, "eiei", "shi'shi"),
            (Scheme::Shoudao, "wlgo", "wai'guo"),
            (Scheme::Shoudao, "erqr", "er'qie"),
            (Scheme::Shoudao, "nvhl", "nv'hai"),
            (Scheme::Shoudao, "lbjl", "lve'jue"),
            (Scheme::Xiaolang, "nihs", "ni'hao"),
            (Scheme::Xiaolang, "elgo", "zhong'guo"),
            (Scheme::Xiaolang, "vzpd", "shuang'pin"),
            (Scheme::Xiaolang, "xbxi", "xue'xi"),
            (Scheme::Xiaolang, "vivi", "shi'shi"),
            (Scheme::Xiaolang, "wkgo", "wai'guo"),
            (Scheme::Xiaolang, "urqp", "er'qie"),
            (Scheme::Xiaolang, "nxhk", "nv'hai"),
            (Scheme::Xiaolang, "ahnx", "ang'nv"),
        ];
        for (scheme, keys, expected) in cases {
            assert_eq!(scheme.decode(keys).pinyin(), expected, "{scheme}: {keys}");
        }
        // 搜狗的 v 不兼作 üe
        assert_eq!(Scheme::Sogou.decode("lv").pinyin(), "");
        assert_eq!(Scheme::Sogou.decode("lv").tail(), "lv");
        // 小浪中 x 為 ü，v 為 sh 或 uai/ing
        assert_eq!(Scheme::Xiaolang.decode("lx").pinyin(), "lv");
        assert_eq!(Scheme::Xiaolang.decode("lv").pinyin(), "ling");
        // 智慧 ABC 的翹舌聲母在 a / e / v；零聲母只認 o 字首，`aa` / `ee` 是 zha / che
        assert_eq!(Scheme::Abc.decode("ai").pinyin(), "zhi");
        assert_eq!(Scheme::Abc.decode("ei").pinyin(), "chi");
        assert_eq!(Scheme::Abc.decode("vi").pinyin(), "shi");
        assert_eq!(Scheme::Abc.decode("oa").pinyin(), "a");
        assert_eq!(Scheme::Abc.decode("oe").pinyin(), "e");
        assert_eq!(Scheme::Abc.decode("aa").pinyin(), "zha");
        assert_eq!(Scheme::Abc.decode("ee").pinyin(), "che");
        // 首道的 sh 在 e：e / ei / eng 用 u 引導，en / er 照全拼敲，ang 是 ay
        assert_eq!(Scheme::Shoudao.decode("ue").pinyin(), "e");
        assert_eq!(Scheme::Shoudao.decode("ui").pinyin(), "ei");
        assert_eq!(Scheme::Shoudao.decode("uf").pinyin(), "eng");
        assert_eq!(Scheme::Shoudao.decode("en").pinyin(), "en");
        assert_eq!(Scheme::Shoudao.decode("er").pinyin(), "er");
        assert_eq!(Scheme::Shoudao.decode("ay").pinyin(), "ang");
        assert_eq!(Scheme::Shoudao.decode("ek").pinyin(), "shen");
        assert_eq!(Scheme::Shoudao.decode("jl").pinyin(), "jue");
        assert_eq!(Scheme::Shoudao.decode("ll").pinyin(), "lai");
        assert_eq!(Scheme::Shoudao.decode("lb").pinyin(), "lve");
    }

    #[test]
    fn trailing_single_key_is_a_prefix() {
        let decoded = Scheme::Xiaohe.decode("kdf");
        assert_eq!(decoded.pinyin(), "kai'f");
        assert!(!decoded.is_complete());
        assert_eq!(Scheme::Xiaohe.decode("v").pinyin(), "zh");
        assert_eq!(Scheme::Xiaohe.decode("a").pinyin(), "a");
        assert_eq!(Scheme::Xiaolang.decode("e").pinyin(), "zh");
        assert_eq!(Scheme::Xiaolang.decode("i").pinyin(), "ch");
        assert_eq!(Scheme::Xiaolang.decode("v").pinyin(), "sh");
        assert_eq!(Scheme::Xiaolang.decode("u").pinyin(), "e");
        assert_eq!(Scheme::Abc.decode("a").pinyin(), "zh");
        assert_eq!(Scheme::Abc.decode("e").pinyin(), "ch");
        assert_eq!(Scheme::Abc.decode("v").pinyin(), "sh");
        assert_eq!(Scheme::Abc.decode("o").pinyin(), "o");
        assert_eq!(Scheme::Shoudao.decode("e").pinyin(), "sh");
        assert_eq!(Scheme::Shoudao.decode("u").pinyin(), "e");
        // `;` 落單不是任何東西
        assert_eq!(Scheme::Microsoft.decode(";").pinyin(), "");
        assert_eq!(Scheme::Microsoft.decode(";").tail(), ";");
    }

    #[test]
    fn invalid_pair_starts_the_tail() {
        let decoded = Scheme::Xiaohe.decode("nibl");
        assert_eq!(decoded.pinyin(), "ni");
        assert_eq!(decoded.tail(), "bl");
        assert_eq!(decoded.marked(), "ni'bl");
    }

    #[test]
    fn apostrophe_ends_a_pair() {
        let decoded = Scheme::Xiaohe.decode("x'ni");
        assert_eq!(decoded.pinyin(), "x'ni");
        assert_eq!(decoded.units()[1].keys, "'");
        assert_eq!(Scheme::Xiaohe.decode("ni'hc").pinyin(), "ni'hao");
    }

    #[test]
    fn parses_config_keys() {
        for scheme in Scheme::ALL {
            assert_eq!(scheme.key().parse::<Scheme>().unwrap(), scheme);
        }
        assert_eq!("Xiaohe ".parse::<Scheme>().unwrap(), Scheme::Xiaohe);
        assert!("flypy".parse::<Scheme>().is_err());
        assert!(Scheme::Microsoft.uses_semicolon());
        assert!(!Scheme::Xiaohe.uses_semicolon());
        assert!(!Scheme::Abc.uses_semicolon());
    }
}
