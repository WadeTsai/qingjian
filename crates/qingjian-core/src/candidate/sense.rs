use serde::{Deserialize, Serialize};

use super::part_of_speech::PartOfSpeech;

/// 一條釋義：詞性 + 目標語言文本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sense {
    /// 該釋義下的詞性。資料來源沒有詞性資訊時為 `None`（如 CC-CEDICT），候選框只顯示譯文。
    pub part_of_speech: Option<PartOfSpeech>,

    /// 學習語言的譯文，短語級別，不是整段解釋。
    pub text: String,

    /// 譯文的讀音（日語假名），給不認識漢字讀法的人看；英文等沒有。
    pub reading: Option<String>,

    /// 生詞：這條譯詞使用者在候選裡還沒見過幾輪（`Engine::annotate` 按詞彙記錄填，釋義表裡恆為 false），殼可以標出來。
    #[serde(default)]
    pub fresh: bool,
}

impl Sense {
    /// 譯文按漢欄位配上假名（振り仮名）；沒有讀音時只有譯文本身一段。
    pub fn furigana(&self) -> Vec<super::FuriganaSegment> {
        match &self.reading {
            Some(reading) => super::furigana(&self.text, reading),
            None => vec![super::FuriganaSegment {
                text: self.text.clone(),
                reading: None,
            }],
        }
    }
}
