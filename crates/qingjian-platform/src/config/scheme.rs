//! 拼音側方案：全拼 / 雙拼七套 / 注音 / 關。配置項 `[general] scheme` 的值。
//! 注音的鍵盤排列（大千 / 倚天）是另一個配置項 `[general] zhuyin_layout`，不在方案裡分開列。
//!
//! 「輸入方案是配置項，不是模式」：中英切換始終是布林，換方案不改變別的方案的既定按鍵行為。
//!
//! **拼音與形碼是兩條獨立的軸**：這裡只管拼音側（讀法），形碼側（五筆）看 `[general] wubi`。
//! 兩邊都開就是混輸，所以舊版那個「單選的 `scheme`」表達不了，2026-09-16 拆開——
//! 當時五筆是 `scheme = "wubi86"`，等價於「拼音關 + 五筆開」。

use std::fmt;
use std::str::FromStr;

use qingjian_core::ShuangpinScheme;

/// 拼音側方案。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scheme {
    /// 全拼。
    #[default]
    Pinyin,

    /// 雙拼，七套鍵位見 [`ShuangpinScheme`]。
    Shuangpin(ShuangpinScheme),

    /// 注音（鍵盤排列另配）。
    Zhuyin,

    /// 關：拼音側不參與查詢，只用形碼。形碼也關著的話沒有候選。
    Off,
}

impl Scheme {
    /// 全部方案，設定介面與狀態條按這個順序列。
    pub const ALL: [Self; 10] = [
        Self::Pinyin,
        Self::Shuangpin(ShuangpinScheme::Xiaohe),
        Self::Shuangpin(ShuangpinScheme::Ziranma),
        Self::Shuangpin(ShuangpinScheme::Microsoft),
        Self::Shuangpin(ShuangpinScheme::Sogou),
        Self::Shuangpin(ShuangpinScheme::Abc),
        Self::Shuangpin(ShuangpinScheme::Xiaolang),
        Self::Shuangpin(ShuangpinScheme::Shoudao),
        Self::Zhuyin,
        Self::Off,
    ];

    /// 配置檔案裡的寫法。`const`：設定介面按 [`Self::ALL`] 直接建常量表，不再手抄一份。
    pub const fn key(self) -> &'static str {
        match self {
            Self::Pinyin => "pinyin",
            Self::Shuangpin(scheme) => scheme.key(),
            Self::Zhuyin => "zhuyin",
            Self::Off => "none",
        }
    }

    /// 介面上的名字。`const` 的理由同上。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pinyin => "全拼",
            Self::Shuangpin(scheme) => scheme.label(),
            Self::Zhuyin => "注音",
            Self::Off => "关（只用形码）",
        }
    }

    /// 這套方案是雙拼時是哪一套；裝配引擎用（不是雙拼時為 `None`）。
    pub const fn shuangpin(self) -> Option<ShuangpinScheme> {
        match self {
            Self::Shuangpin(scheme) => Some(scheme),
            _ => None,
        }
    }

    /// 這套方案是否為雙拼方案。
    pub const fn is_shuangpin(self) -> bool {
        matches!(self, Self::Shuangpin(_))
    }

    /// 拼音側參不參與查詢。
    pub const fn is_on(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// 日誌裡的寫法：全拼為空串（老日誌裡沒有這個欄位就是全拼），其餘同 [`Self::key`]。
    pub const fn log_key(self) -> &'static str {
        match self {
            Self::Pinyin => "",
            other => other.key(),
        }
    }
}

/// 狀態條上顯示的方案名，形碼在前（與候選順序一致）；全拼且不開形碼時為空串。
/// 全拼只在同時開著形碼時才寫出來——單開全拼是預設，標它沒意義。
///
/// 做成跟 [`Scheme`] 與形碼開關一起算的自由函式，而不是存進 `RouterConfig`：
/// 存下來的話它會與那兩項冗餘、手搓配置的地方就會漂移（測試正是這麼發現的）。
pub fn scheme_label(pinyin: Scheme, wubi: bool) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if wubi {
        parts.push("五笔（86）");
    }
    if pinyin.is_on() && (wubi || pinyin != Scheme::Pinyin) {
        parts.push(pinyin.label());
    }
    parts.join(" + ")
}

/// 主題顯示的方案名：同 [`scheme_label`]，只是單開全拼時也寫出「全拼」（主題要一個總有內容的名字）。
pub fn scheme_name(pinyin: Scheme, wubi: bool) -> String {
    let label = scheme_label(pinyin, wubi);
    if label.is_empty() {
        pinyin.label().to_owned()
    } else {
        label
    }
}

impl FromStr for Scheme {
    type Err = String;

    /// 認不出來的寫法報錯，由呼叫方決定退回什麼（配置層退回全拼並警告）。
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim() {
            "" | "pinyin" => Ok(Self::Pinyin),
            "zhuyin" => Ok(Self::Zhuyin),
            // 舊配置把五筆寫在這一欄（2026-09-16 之前），等價於「拼音關」
            "wubi" | "wubi86" => Ok(Self::Off),
            "none" | "off" => Ok(Self::Off),
            other => other.parse().map(Self::Shuangpin),
        }
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

    #[test]
    fn scheme_name_always_says_something() {
        assert_eq!(scheme_name(Scheme::Pinyin, false), "全拼");
        assert_eq!(scheme_name(Scheme::Zhuyin, false), "注音");
        assert_eq!(
            scheme_name(Scheme::Pinyin, true),
            scheme_label(Scheme::Pinyin, true)
        );
    }

    #[test]
    fn parses_every_key_it_prints() {
        for scheme in Scheme::ALL {
            assert_eq!(scheme.key().parse::<Scheme>(), Ok(scheme), "{scheme}");
        }
    }

    #[test]
    fn empty_and_unknown_spellings() {
        assert_eq!("".parse::<Scheme>(), Ok(Scheme::Pinyin));
        assert_eq!(" pinyin ".parse::<Scheme>(), Ok(Scheme::Pinyin));
        assert_eq!("none".parse::<Scheme>(), Ok(Scheme::Off));
        assert_eq!("off".parse::<Scheme>(), Ok(Scheme::Off));
        assert!("flypy".parse::<Scheme>().is_err());
    }

    #[test]
    fn the_old_wubi_spelling_means_turn_the_pinyin_side_off() {
        // 2026-09-16 之前五筆是 `scheme = "wubi86"`，那時它是一個單選的方案
        assert_eq!("wubi86".parse::<Scheme>(), Ok(Scheme::Off));
        assert_eq!("wubi".parse::<Scheme>(), Ok(Scheme::Off));
        assert!(!Scheme::Off.is_on());
        assert!(Scheme::Pinyin.is_on());
    }
}
