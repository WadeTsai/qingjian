//! 候選視窗一次繪製的全部內容。視窗記住上一幀，聯想結果到了只改一處再重畫。

use super::preedit::Preedit;
use super::row::Row;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    /// 頂部拼音行；配置成只在行內顯示時為 `None`。
    pub preedit: Option<Preedit>,

    /// 候選行。
    pub rows: Vec<Row>,

    /// 高亮行下標；不想高亮任何行就給 `usize::MAX`。
    pub highlighted: usize,

    /// 橫排展開成矩陣時每行幾格，`rows` 按行優先排開、空位是空行；0 為沒展開。
    pub columns: usize,

    /// 矩陣各列要留幾個候選字寬（按整份候選估的，滾動、移動高亮時不變，窗口才不跳）。
    pub column_ems: Vec<f32>,

    /// 右下角頁碼。
    pub footer: Option<String>,

    /// 拼音行右側的整句補全（雲聯想），組句時才有。
    pub sentence: Option<String>,

    /// 拼音行右側的一句臨時狀態（刪了什麼詞），有它時不畫整句補全。
    pub status: Option<String>,

    /// 輸入狀態，青簡渲染器的主題拿來顯示中 / 英、簡 / 繁等；系統繪製不用。
    pub mode: qingjian_render::Mode,
}

impl Frame {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.preedit.is_none() && self.trailing().is_none()
    }

    /// 頂部要不要畫一行（拼音或右側文字任一存在）。
    pub fn has_top_line(&self) -> bool {
        self.preedit.is_some() || self.trailing().is_some()
    }

    /// 拼音行右側畫什麼：狀態優先，其次整句補全；`bool` 是要不要帶雲朵。
    pub fn trailing(&self) -> Option<(&str, bool)> {
        self.status
            .as_deref()
            .map(|s| (s, false))
            .or_else(|| self.sentence.as_deref().map(|s| (s, true)))
    }
}
