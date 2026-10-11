/// 檔案裡裝的是哪種資料。編號寫進檔案頭，不要改已有編號。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Kind {
    /// 拼音詞庫（`qingjian-dictionary::Dictionary`）。
    Dictionary = 1,

    /// 詞級 bigram 語言模型（`qingjian-lm::BigramModel`）。
    LanguageModel = 2,

    /// 釋義表。
    Glossary = 3,

    /// emoji 表。
    Emoji = 4,

    /// 英文詞表。
    WordList = 5,

    /// 本地整句模型（`qingjian-neural::CharScorer`，副檔名 `.qjm`）。
    Model = 6,

    /// 輔碼碼錶（`qingjian-dictionary::AuxCodeTable`）：詞 → 碼的對映，輔碼態逐鍵即篩用。
    AuxCodeTable = 7,
}

impl Kind {
    pub fn from_code(code: u16) -> Option<Self> {
        Some(match code {
            1 => Self::Dictionary,
            2 => Self::LanguageModel,
            3 => Self::Glossary,
            4 => Self::Emoji,
            5 => Self::WordList,
            6 => Self::Model,
            7 => Self::AuxCodeTable,
            _ => return None,
        })
    }

    pub fn code(self) -> u16 {
        self as u16
    }
}
