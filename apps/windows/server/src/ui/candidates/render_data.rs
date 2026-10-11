//! 候選視窗一次繪製要用的全部內容，由幀換算而來；渲染器要的幀由 [`RenderData::render_frame`] 再換一次。

use std::rc::Rc;

use qingjian_platform::protocol::{Frame, PreeditKind};
use qingjian_platform::{Appearance, LayoutMode};
use qingjian_render::{Mode, Preedit, PreeditSegment, PreeditStyle, Row};

use super::row;
use super::theme::Theme;

/// 一次繪製要用的全部內容。
pub(crate) struct RenderData {
    /// 配色與字型（隨 DPI / 深淺重建）。
    pub(super) theme: Rc<Theme>,

    /// 頂部拼音行的各段。
    pub(super) preedit: Vec<(String, PreeditKind)>,

    /// 游標在拼音行裡的字元位置。
    pub(super) cursor: usize,

    /// 候選行。
    pub(super) rows: Vec<Row>,

    /// 高亮行下標（頁內）。
    pub(super) highlight: usize,

    /// 頁碼，只有多頁時有。
    pub(super) footer: Option<String>,

    /// 整句補全，畫在拼音行右側。
    pub(super) sentence: Option<String>,

    /// 螢幕提示（刪候選後的「已刪除…」），畫在拼音行下方。
    pub(super) notice: Option<String>,

    /// 候選排布。
    pub(super) layout: LayoutMode,

    /// 外觀；`System` 由視窗按系統深淺色解析。
    pub(super) appearance: Appearance,

    /// 候選上是否顯示輔碼（隨幀下發的 `[general] aux_code_show`）。
    pub(super) show_code: bool,

    /// 輸入狀態，給主題顯示中 / 英、簡 / 繁等。
    pub(super) mode: Mode,
}

impl RenderData {
    pub(super) fn empty(theme: Rc<Theme>) -> Self {
        Self {
            theme,
            preedit: Vec::new(),
            cursor: 0,
            rows: Vec::new(),
            highlight: usize::MAX,
            footer: None,
            sentence: None,
            notice: None,
            layout: LayoutMode::default(),
            appearance: Appearance::default(),
            show_code: false,
            mode: Mode::default(),
        }
    }

    pub(super) fn set(&mut self, frame: &Frame, mode: Mode) {
        self.mode = mode;
        self.layout = frame.layout;
        self.appearance = frame.appearance;
        self.show_code = frame.aux_code_show;
        self.preedit = window_preedit(frame);
        self.cursor = frame.cursor;
        self.rows = frame
            .candidates
            .items
            .iter()
            .enumerate()
            .map(|(i, candidate)| row::from_candidate(i, candidate, self.show_code))
            .collect();
        self.highlight = frame.highlight;
        self.footer =
            (frame.page_count > 1).then(|| format!("{}/{}", frame.page + 1, frame.page_count));
        self.sentence = frame.sentence.clone();
        self.notice = frame.notice.clone();
    }

    /// 渲染器要的幀。提示（刪了什麼詞）在渲染器裡畫在拼音行右側，與 macOS 一致。
    pub(super) fn render_frame(&self) -> qingjian_render::Frame {
        let preedit = (!self.preedit.is_empty()).then(|| Preedit {
            segments: self
                .preedit
                .iter()
                .map(|(text, kind)| PreeditSegment {
                    text: text.clone(),
                    style: match kind {
                        PreeditKind::Typed => PreeditStyle::Typed,
                        PreeditKind::Rest => PreeditStyle::Rest,
                        PreeditKind::Corrected => PreeditStyle::Struck,
                        PreeditKind::AuxCode => PreeditStyle::AuxCode,
                    },
                })
                .collect(),
            cursor: self.cursor,
        });
        qingjian_render::Frame {
            preedit,
            rows: self.rows.clone(),
            // 協議裡 usize::MAX 表示不高亮。
            highlighted: (self.highlight != usize::MAX).then_some(self.highlight),
            columns: 0,
            column_ems: Vec::new(),
            footer: self.footer.clone(),
            sentence: self.sentence.clone(),
            status: self.notice.clone(),
            mode: self.mode.clone(),
        }
    }
}

/// 視窗頂部要畫的拼音行：`[general] preedit` 配成「只在行內」時為空（拼音已經在應用裡）；
/// 整句補全仍畫在這一行上，所以空行時視窗仍可能留出這條線。
fn window_preedit(frame: &Frame) -> Vec<(String, PreeditKind)> {
    if !frame.preedit_mode.in_window() {
        return Vec::new();
    }
    frame
        .preedit
        .iter()
        .map(|segment| (segment.text.clone(), segment.kind))
        .collect()
}

#[cfg(test)]
mod tests {
    use qingjian_platform::PreeditMode;
    use qingjian_platform::protocol::{Frame, PreeditKind, PreeditSegment};

    use super::window_preedit;

    fn frame(mode: PreeditMode) -> Frame {
        Frame {
            preedit: vec![PreeditSegment {
                text: "ni'hao".to_owned(),
                kind: PreeditKind::Typed,
            }],
            preedit_mode: mode,
            ..Frame::default()
        }
    }

    /// 只有「只在行內」不給視窗拼音行；另兩檔視窗都得畫。
    #[test]
    fn window_keeps_the_pinyin_row_unless_inline_only() {
        assert_eq!(window_preedit(&frame(PreeditMode::Both)).len(), 1);
        assert_eq!(window_preedit(&frame(PreeditMode::Window)).len(), 1);
        assert!(window_preedit(&frame(PreeditMode::Inline)).is_empty());
    }
}
