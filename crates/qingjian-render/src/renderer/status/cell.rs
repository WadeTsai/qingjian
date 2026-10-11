//! 狀態條的一格。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusCell {
    /// 一段文字；`emphasized` 用品牌色（當前模式、生效中的全形標點），否則用譯文的灰。
    Text { text: String, emphasized: bool },

    /// 開啟設定的齒輪。
    Gear,
}

impl StatusCell {
    pub fn text(text: impl Into<String>, emphasized: bool) -> Self {
        Self::Text {
            text: text.into(),
            emphasized,
        }
    }
}
