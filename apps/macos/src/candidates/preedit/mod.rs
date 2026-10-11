mod segment;
mod style;

use qingjian_core::MarkedSegment;

use segment::PreeditSegment;
pub use style::PreeditStyle;

/// 候選視窗頂部的拼音行：若干段 + 我們自己畫的游標。游標位置是各段拼接後的字元下標。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preedit {
    /// 按順序畫的片段。
    pub segments: Vec<PreeditSegment>,

    /// 游標在拼接文本里的字元位置。
    pub cursor: usize,
}

impl Preedit {
    /// 由 Core 的分段與游標位置構造；沒有任何文字時返回 `None`。
    pub fn from_marked(segments: &[MarkedSegment], cursor: usize) -> Option<Self> {
        let segments: Vec<PreeditSegment> = segments
            .iter()
            .filter(|s| !s.text.is_empty())
            .map(PreeditSegment::from)
            .collect();
        (!segments.is_empty()).then_some(Self { segments, cursor })
    }

    /// 單段普通文本（查詢失敗時退回顯示原始字母）。
    pub fn plain(text: &str, cursor: usize) -> Option<Self> {
        (!text.is_empty()).then(|| Self {
            segments: vec![PreeditSegment {
                text: text.to_owned(),
                style: PreeditStyle::Typed,
            }],
            cursor,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::MarkedKind;

    #[test]
    fn builds_from_marked_segments_and_concatenates() {
        let segments = [
            MarkedSegment::new("ni'hao", MarkedKind::Typed),
            MarkedSegment::new("", MarkedKind::Rest),
            MarkedSegment::new("'ma", MarkedKind::Rest),
        ];
        let preedit = Preedit::from_marked(&segments, 6).unwrap();
        assert_eq!(preedit.segments.len(), 2);
        assert_eq!(preedit.segments[1].style, PreeditStyle::Rest);
        assert_eq!(preedit.text(), "ni'hao'ma");
        assert_eq!(preedit.before_cursor(), "ni'hao");
        assert!(Preedit::from_marked(&[], 0).is_none());
        assert!(Preedit::plain("", 0).is_none());
    }
}
