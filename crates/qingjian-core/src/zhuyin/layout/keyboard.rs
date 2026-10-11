//! 注音鍵盤排列：大千（標準）與倚天。配置項 `[general] zhuyin_layout` 的值；兩種排列解出同樣的符號，只是鍵不同。
//!
//! 鍵位表按注音符號的固定順序寫成一串（與 libchewing 的 `keytab` 同一寫法，方便對照）：
//! 21 個聲母、3 個介音、13 個韻母、4 個聲調（˙ ˊ ˇ ˋ）；一聲在兩種排列下都是空格。

use std::fmt;
use std::str::FromStr;

use super::component::Component;

/// 鍵位表對應的符號，順序固定：ㄅ…ㄙ、ㄧㄨㄩ、ㄚ…ㄦ、˙ˊˇˋ。
const COMPONENTS: [Component; 41] = [
    Component::Initial('ㄅ'),
    Component::Initial('ㄆ'),
    Component::Initial('ㄇ'),
    Component::Initial('ㄈ'),
    Component::Initial('ㄉ'),
    Component::Initial('ㄊ'),
    Component::Initial('ㄋ'),
    Component::Initial('ㄌ'),
    Component::Initial('ㄍ'),
    Component::Initial('ㄎ'),
    Component::Initial('ㄏ'),
    Component::Initial('ㄐ'),
    Component::Initial('ㄑ'),
    Component::Initial('ㄒ'),
    Component::Initial('ㄓ'),
    Component::Initial('ㄔ'),
    Component::Initial('ㄕ'),
    Component::Initial('ㄖ'),
    Component::Initial('ㄗ'),
    Component::Initial('ㄘ'),
    Component::Initial('ㄙ'),
    Component::Medial('ㄧ'),
    Component::Medial('ㄨ'),
    Component::Medial('ㄩ'),
    Component::Final('ㄚ'),
    Component::Final('ㄛ'),
    Component::Final('ㄜ'),
    Component::Final('ㄝ'),
    Component::Final('ㄞ'),
    Component::Final('ㄟ'),
    Component::Final('ㄠ'),
    Component::Final('ㄡ'),
    Component::Final('ㄢ'),
    Component::Final('ㄣ'),
    Component::Final('ㄤ'),
    Component::Final('ㄥ'),
    Component::Final('ㄦ'),
    Component::Tone(5, '˙'),
    Component::Tone(2, 'ˊ'),
    Component::Tone(3, 'ˇ'),
    Component::Tone(4, 'ˋ'),
];

/// 大千（標準）排列：聲母在左、韻母在右，數字鍵與 `- ; , . /` 也是符號 / 聲調。
const DACHEN_KEYS: &str = "1qaz2wsxedcrfv5tgbyhnujm8ik,9ol.0p;/-7634";

/// 倚天排列：聲母按拼音字母記（b p m f d t n l …），韻母在左手，`'` 是 ㄘ、`=` 是 ㄦ，聲調在 1 2 3 4。
const ETEN_KEYS: &str = "bpmfdtnlvkhg7c,./j;'sexuaorwiqzy890-=1234";

/// 注音鍵盤排列。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ZhuyinLayout {
    /// 大千，也叫標準排列；絕大多數注音使用者用它。
    #[default]
    Dachen,

    /// 倚天排列（倚天中文系統的鍵盤）。
    Eten,
}

impl ZhuyinLayout {
    /// 全部排列，設定介面按這個順序列。
    pub const ALL: [Self; 2] = [Self::Dachen, Self::Eten];

    /// 配置檔案裡的寫法。
    pub const fn key(self) -> &'static str {
        match self {
            Self::Dachen => "dachen",
            Self::Eten => "eten",
        }
    }

    /// 介面上的名字。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dachen => "標準（大千）",
            Self::Eten => "倚天",
        }
    }

    const fn keys(self) -> &'static str {
        match self {
            Self::Dachen => DACHEN_KEYS,
            Self::Eten => ETEN_KEYS,
        }
    }

    /// 把一個按鍵解成注音符號元件；不是這套排列的鍵為 `None`。空格在哪套排列下都是一聲。
    pub fn map_key(self, key: char) -> Option<Component> {
        if key == ' ' {
            // 一聲不顯示標記
            return Some(Component::Tone(1, ' '));
        }
        if !key.is_ascii() {
            return None;
        }
        self.keys()
            .bytes()
            .position(|k| k == key as u8)
            .map(|index| COMPONENTS[index])
    }

    /// 這個鍵在這套排列下是不是符號 / 聲調鍵（空格不算：殼對空格另有處理）。
    pub fn is_key(self, key: char) -> bool {
        key != ' ' && self.map_key(key).is_some()
    }
}

impl FromStr for ZhuyinLayout {
    type Err = String;

    /// 認不出來的寫法報錯，由呼叫方決定退回什麼（配置層退回大千並警告）。
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim() {
            "" | "dachen" | "standard" => Ok(Self::Dachen),
            "eten" => Ok(Self::Eten),
            other => Err(format!("unknown zhuyin layout: {other}")),
        }
    }
}

impl fmt::Display for ZhuyinLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

#[cfg(test)]
mod tests {
    use super::{COMPONENTS, Component, DACHEN_KEYS, ETEN_KEYS, ZhuyinLayout};

    #[test]
    fn both_tables_cover_every_component_exactly_once() {
        for keys in [DACHEN_KEYS, ETEN_KEYS] {
            assert_eq!(keys.len(), COMPONENTS.len());
            let mut seen = keys.bytes().collect::<Vec<_>>();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), COMPONENTS.len(), "{keys}");
        }
    }

    #[test]
    fn dachen_keeps_the_standard_positions() {
        let layout = ZhuyinLayout::Dachen;
        assert_eq!(layout.map_key('1'), Some(Component::Initial('ㄅ')));
        assert_eq!(layout.map_key('d'), Some(Component::Initial('ㄎ')));
        assert_eq!(layout.map_key('5'), Some(Component::Initial('ㄓ')));
        assert_eq!(layout.map_key('n'), Some(Component::Initial('ㄙ')));
        assert_eq!(layout.map_key('u'), Some(Component::Medial('ㄧ')));
        assert_eq!(layout.map_key('m'), Some(Component::Medial('ㄩ')));
        assert_eq!(layout.map_key('8'), Some(Component::Final('ㄚ')));
        assert_eq!(layout.map_key(','), Some(Component::Final('ㄝ')));
        assert_eq!(layout.map_key('9'), Some(Component::Final('ㄞ')));
        assert_eq!(layout.map_key('/'), Some(Component::Final('ㄥ')));
        assert_eq!(layout.map_key('-'), Some(Component::Final('ㄦ')));
        assert_eq!(layout.map_key('7'), Some(Component::Tone(5, '˙')));
        assert_eq!(layout.map_key('6'), Some(Component::Tone(2, 'ˊ')));
        assert_eq!(layout.map_key('3'), Some(Component::Tone(3, 'ˇ')));
        assert_eq!(layout.map_key('4'), Some(Component::Tone(4, 'ˋ')));
        assert_eq!(layout.map_key(' '), Some(Component::Tone(1, ' ')));
        assert_eq!(layout.map_key('\''), None);
        assert_eq!(layout.map_key('='), None);
        assert_eq!(layout.map_key('ㄅ'), None);
    }

    #[test]
    fn eten_uses_pinyin_letters_for_initials_and_digits_for_tones() {
        let layout = ZhuyinLayout::Eten;
        assert_eq!(layout.map_key('b'), Some(Component::Initial('ㄅ')));
        assert_eq!(layout.map_key('k'), Some(Component::Initial('ㄎ')));
        assert_eq!(layout.map_key('v'), Some(Component::Initial('ㄍ')));
        assert_eq!(layout.map_key('7'), Some(Component::Initial('ㄑ')));
        assert_eq!(layout.map_key(','), Some(Component::Initial('ㄓ')));
        assert_eq!(layout.map_key('\''), Some(Component::Initial('ㄘ')));
        assert_eq!(layout.map_key('e'), Some(Component::Medial('ㄧ')));
        assert_eq!(layout.map_key('x'), Some(Component::Medial('ㄨ')));
        assert_eq!(layout.map_key('u'), Some(Component::Medial('ㄩ')));
        assert_eq!(layout.map_key('a'), Some(Component::Final('ㄚ')));
        assert_eq!(layout.map_key('i'), Some(Component::Final('ㄞ')));
        assert_eq!(layout.map_key('='), Some(Component::Final('ㄦ')));
        assert_eq!(layout.map_key('1'), Some(Component::Tone(5, '˙')));
        assert_eq!(layout.map_key('2'), Some(Component::Tone(2, 'ˊ')));
        assert_eq!(layout.map_key('4'), Some(Component::Tone(4, 'ˋ')));
        assert_eq!(layout.map_key(' '), Some(Component::Tone(1, ' ')));
        assert_eq!(layout.map_key('5'), None);
        assert_eq!(layout.map_key('6'), None);
        assert!(!layout.is_key(' '));
        assert!(layout.is_key('='));
    }

    #[test]
    fn parses_every_key_it_prints() {
        for layout in ZhuyinLayout::ALL {
            assert_eq!(layout.key().parse::<ZhuyinLayout>(), Ok(layout));
        }
        assert_eq!("".parse::<ZhuyinLayout>(), Ok(ZhuyinLayout::Dachen));
        assert_eq!("standard".parse::<ZhuyinLayout>(), Ok(ZhuyinLayout::Dachen));
        assert!("hsu".parse::<ZhuyinLayout>().is_err());
    }
}
