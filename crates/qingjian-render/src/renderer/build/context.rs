//! 例項化時的資料上下文：整幀，加上 `repeat` 展開中的那一項（候選，或狀態條的一格）。顯示條件（`when`）與繫結（`bind`）都在這裡求值。
//!
//! 名字先在候選項裡找，再在整幀裡找；不認識的條件為假、不認識的繫結為空（節點不畫）。

use super::cell::CellContext;
use super::row::RowContext;
use crate::frame::{Frame, Row, Tone};
use crate::renderer::StatusCell;

#[derive(Debug, Clone, Copy)]
pub(super) struct Context<'a> {
    pub(super) frame: &'a Frame,

    /// 在 `repeat` 裡時繫結的那一項。
    pub(super) row: Option<RowContext<'a>>,

    /// 狀態條的格子（`repeat` 繫結 `cells`）；畫候選視窗時為空。
    pub(super) cells: &'a [StatusCell],

    /// 在 `repeat` 裡時繫結的那一格。
    pub(super) cell: Option<CellContext<'a>>,
}

impl<'a> Context<'a> {
    pub(super) fn new(frame: &'a Frame) -> Self {
        Self {
            frame,
            row: None,
            cells: &[],
            cell: None,
        }
    }

    /// 狀態條：整幀只用到輸入狀態，格子供 `repeat` 展開。
    pub(super) fn status(frame: &'a Frame, cells: &'a [StatusCell]) -> Self {
        Self {
            cells,
            ..Self::new(frame)
        }
    }

    /// 展開格子時每一格的上下文。
    pub(super) fn with_cell(self, cell: &'a StatusCell, index: usize, count: usize) -> Self {
        Self {
            cell: Some(CellContext { cell, index, count }),
            ..self
        }
    }

    /// 展開列表時每一項的上下文。
    pub(super) fn with_row(self, row: &'a Row, index: usize, count: usize) -> Self {
        Self {
            row: Some(RowContext { row, index, count }),
            ..self
        }
    }

    /// `when` 求值：`a|b` 任一成立，`!a` 取反。
    pub(super) fn holds(&self, condition: &str) -> bool {
        condition.split('|').any(|term| {
            let term = term.trim();
            match term.strip_prefix('!') {
                Some(name) => !self.flag(name),
                None => self.flag(term),
            }
        })
    }

    fn flag(&self, name: &str) -> bool {
        if let Some(CellContext { cell, index, count }) = self.cell {
            match name {
                "emphasized" => {
                    return matches!(
                        cell,
                        StatusCell::Text {
                            emphasized: true,
                            ..
                        }
                    );
                }
                "gear" => return *cell == StatusCell::Gear,
                "first" => return index == 0,
                "last" => return index + 1 == count,
                _ => {}
            }
        }
        if let Some(RowContext { row, index, count }) = self.row {
            match name {
                "highlighted" => return self.frame.highlighted == Some(index),
                "cloud" => return row.cloud,
                "annotation" => return !row.annotation.is_empty(),
                "code" => return row.code.is_some(),
                "first" => return index == 0,
                "last" => return index + 1 == count,
                _ => {}
            }
        }
        let frame = self.frame;
        match name {
            "preedit" => frame.preedit.is_some(),
            "trailing" => frame.trailing().is_some(),
            "trailing.cloud" => frame.trailing().is_some_and(|(_, cloud)| cloud),
            "page" => frame.footer.is_some(),
            "candidates" => !frame.rows.is_empty(),
            "annotations" => frame.rows.iter().any(|row| !row.annotation.is_empty()),
            "highlighted" => self.highlighted().is_some(),
            "highlighted.annotation" => self
                .highlighted()
                .is_some_and(|row| !row.annotation.is_empty()),
            "mode.english" => frame.mode.english,
            "mode.traditional" => frame.mode.traditional,
            "mode.full_width" => frame.mode.full_width,
            "mode.scheme" => !frame.mode.scheme.is_empty(),
            _ => false,
        }
    }

    /// 文字繫結。
    pub(super) fn text(&self, name: &str) -> Option<&'a str> {
        if let Some(CellContext {
            cell: StatusCell::Text { text, .. },
            ..
        }) = self.cell
            && name == "text"
        {
            return Some(text);
        }
        if let Some(RowContext { row, .. }) = self.row {
            match name {
                "index" => return Some(&row.index),
                "text" => return Some(&row.text),
                "code" => return row.code.as_deref(),
                _ => {}
            }
        }
        match name {
            "page" => self.frame.footer.as_deref(),
            "trailing.text" => self.frame.trailing().map(|(text, _)| text),
            "mode.scheme" => Some(self.frame.mode.scheme.as_str()).filter(|s| !s.is_empty()),
            _ => None,
        }
    }

    /// 譯文片段繫結。
    pub(super) fn annotation(&self, name: &str) -> Option<&'a [(String, Tone)]> {
        match (name, self.row) {
            ("annotation", Some(RowContext { row, .. })) => Some(&row.annotation),
            ("highlighted.annotation", _) => {
                self.highlighted().map(|row| row.annotation.as_slice())
            }
            _ => None,
        }
    }

    /// 列表繫結。
    pub(super) fn list(&self, name: &str) -> Option<&'a [Row]> {
        (name == "candidates").then_some(self.frame.rows.as_slice())
    }

    fn highlighted(&self) -> Option<&'a Row> {
        self.frame.highlighted.and_then(|i| self.frame.rows.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::Context;
    use crate::frame::{Frame, Mode};

    #[test]
    fn mode_conditions_and_scheme_binding() {
        let frame = Frame {
            mode: Mode {
                english: true,
                traditional: false,
                full_width: true,
                scheme: "小鹤双拼".to_owned(),
            },
            ..Frame::default()
        };
        let ctx = Context::new(&frame);
        assert!(ctx.holds("mode.english") && !ctx.holds("!mode.english"));
        assert!(!ctx.holds("mode.traditional") && ctx.holds("mode.full_width"));
        assert!(ctx.holds("mode.scheme") && !Context::new(&Frame::default()).holds("mode.scheme"));
        assert_eq!(ctx.text("mode.scheme"), Some("小鹤双拼"));
        assert_eq!(Context::new(&Frame::default()).text("mode.scheme"), None);
    }
}
