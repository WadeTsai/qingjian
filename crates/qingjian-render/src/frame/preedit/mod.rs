//! 候選視窗頂部的拼音行：若干段 + 渲染器自己畫的游標。

mod segment;
mod style;

pub use segment::PreeditSegment;
pub use style::PreeditStyle;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preedit {
    /// 按順序畫的片段。
    pub segments: Vec<PreeditSegment>,

    /// 游標在拼接文本里的字元位置。
    pub cursor: usize,
}

impl Preedit {
    /// 單段普通文本。
    pub fn plain(text: &str, cursor: usize) -> Self {
        Self {
            segments: vec![PreeditSegment {
                text: text.to_owned(),
                style: PreeditStyle::Typed,
            }],
            cursor,
        }
    }

    /// 拼接後的全文。
    pub fn text(&self) -> String {
        self.segments.iter().map(|s| s.text.as_str()).collect()
    }

    /// 游標前的文字（用來量游標的 x）。
    pub fn before_cursor(&self) -> String {
        self.text().chars().take(self.cursor).collect()
    }
}
