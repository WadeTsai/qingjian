//! 系統繪製路徑的矩陣排布：橫排展開後的多行網格，規則與渲染器的 `renderer/matrix.rs` 一致——
//! 一行 `frame.columns` 格，各列寬度由幀給（按整份候選估的，滾動時不變），超寬的候選截尾加「…」，
//! 網格下面固定留一行資訊（完整文本、譯文、頁碼），資訊行放不下的也截斷，視窗寬度只由列寬決定。

use objc2_app_kit::{NSColor, NSFont};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::{CandidateView, HIGHLIGHT_INSET, INDEX_GAP};
use crate::candidates::frame::Frame;

/// 幀沒給列寬時，一格里候選詞最多多寬（按候選字號的倍數）。
const MAX_CELL_EMS: f64 = 4.0;

/// 列寬在估出來的字寬之外再留一點：估寬按整字算，字形實際會多出零點幾個點。
const CELL_SLACK: f64 = 1.5;

/// 資訊行裡完整文本與譯文之間的間距。
const INFO_GAP: f64 = 10.0;

const ELLIPSIS: &str = "…";

/// 量好的網格：每格顯示的文字（可能已截斷）、各列的寬度與統一的行高。
struct Cells {
    texts: Vec<(String, bool)>,

    index_width: f64,

    /// 每列一格的寬度（序號 + 間距 + 候選詞）。
    column_widths: Vec<f64>,

    row_height: f64,
}

impl Cells {
    /// 第 `column` 列左邊相對網格起點的偏移。
    fn offset(&self, column: usize, gap: f64) -> f64 {
        self.column_widths[..column].iter().sum::<f64>() + gap * column as f64
    }

    /// 網格總寬（不含兩側高亮留邊）。
    fn width(&self, gap: f64) -> f64 {
        self.column_widths.iter().sum::<f64>()
            + gap * self.column_widths.len().saturating_sub(1) as f64
    }
}

impl CandidateView {
    pub(super) fn matrix_size(&self, frame: &Frame) -> (f64, f64) {
        if frame.rows.is_empty() {
            return (0.0, 0.0);
        }
        let theme = self.theme();
        let cells = self.matrix_cells(frame);
        let grid_rows = frame.rows.len().div_ceil(frame.columns.max(1));
        let info_height = self.measure("x", &theme.annotation_font).height + theme.row_padding;
        (
            cells.width(theme.column_gap) + HIGHLIGHT_INSET * 2.0,
            cells.row_height * grid_rows as f64 + info_height,
        )
    }

    pub(super) fn draw_matrix(&self, frame: &Frame, y: f64, bounds: NSRect) {
        if frame.rows.is_empty() {
            return;
        }
        let theme = self.theme();
        let cells = self.matrix_cells(frame);
        let columns = frame.columns.max(1);
        let text_height = self.measure("x", &theme.text_font).height;
        let origin = theme.padding + HIGHLIGHT_INSET;
        for (i, row) in frame.rows.iter().enumerate() {
            let (text, _) = &cells.texts[i];
            if text.is_empty() && row.index.is_empty() {
                continue;
            }
            let cell_width = cells.column_widths[i % columns];
            let x = origin + cells.offset(i % columns, theme.column_gap);
            let row_y = y + cells.row_height * (i / columns) as f64;
            let baseline = row_y + theme.row_padding;
            if i == frame.highlighted {
                self.fill_highlight(NSRect::new(
                    NSPoint::new(x - HIGHLIGHT_INSET, row_y),
                    NSSize::new(cell_width + HIGHLIGHT_INSET * 2.0, cells.row_height),
                ));
            }
            if !row.index.is_empty() {
                self.draw_text(
                    &row.index,
                    &theme.index_font,
                    &theme.index_color,
                    baseline + self.small_offset(text_height),
                    x,
                );
            }
            let mut shown = row.clone();
            shown.text.clone_from(text);
            self.draw_word(
                &shown,
                x + cells.index_width + INDEX_GAP,
                baseline,
                text_height,
            );
        }
        // 資訊行：頁碼靠右；左邊先放被截斷的高亮候選的完整文本，再放譯文，放不下的截斷
        let grid_rows = frame.rows.len().div_ceil(columns);
        let info_top = y + cells.row_height * grid_rows as f64 + theme.row_padding / 2.0;
        let right = bounds.size.width - theme.padding;
        let mut budget = right - origin;
        if let Some(footer) = frame.footer.as_deref() {
            let size = self.measure(footer, &theme.index_font);
            self.draw_text(
                footer,
                &theme.index_font,
                &theme.index_color,
                info_top,
                right - size.width,
            );
            budget -= size.width + theme.column_gap;
        }
        let Some(row) = frame.rows.get(frame.highlighted) else {
            return;
        };
        let mut x = origin;
        if cells.texts[frame.highlighted].1 {
            let used = self.draw_clipped(&row.text, &theme.text_color, x, info_top, budget);
            x += used + INFO_GAP;
            budget -= used + INFO_GAP;
        }
        for (segment, tone) in &row.annotation {
            if budget <= 0.0 {
                break;
            }
            let used = self.draw_clipped(segment, self.tone_color(*tone), x, info_top, budget);
            x += used;
            budget -= used;
        }
    }

    /// 在資訊行裡畫一段小字，寬度超過 `budget` 就截斷；返回畫了多寬。
    fn draw_clipped(&self, text: &str, color: &NSColor, x: f64, top: f64, budget: f64) -> f64 {
        let font = &self.theme().annotation_font;
        let (shown, _) = self.truncate(text, font, budget);
        self.draw_text(&shown, font, color, top, x)
    }

    /// 每格的顯示文字與各列寬度。序號列按最寬的一位數留，行與行才對得齊。
    /// 列寬優先用幀給的（按整份候選估的，滾動時不變）；沒給就按視口裡實測、每格封頂 [`MAX_CELL_EMS`]。
    fn matrix_cells(&self, frame: &Frame) -> Cells {
        let theme = self.theme();
        let columns = frame.columns.max(1);
        let em = theme.text_font.pointSize();
        let index_width = self.measure("8", &theme.index_font).width;
        let fixed: Option<Vec<f64>> = (frame.column_ems.len() == columns).then(|| {
            frame
                .column_ems
                .iter()
                .map(|ems| f64::from(*ems) * em + CELL_SLACK)
                .collect()
        });
        let mut text_widths = fixed.clone().unwrap_or_else(|| vec![0.0; columns]);
        let mut row_height: f64 = 0.0;
        let texts = frame
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let column = i % columns;
                let cloud = if row.cloud { self.cloud_width() } else { 0.0 };
                let limit = fixed
                    .as_ref()
                    .map_or(em * MAX_CELL_EMS, |widths| widths[column]);
                let (text, truncated) = self.truncate(&row.text, &theme.text_font, limit - cloud);
                let size = self.measure(&text, &theme.text_font);
                if fixed.is_none() {
                    text_widths[column] = text_widths[column].max(size.width + cloud);
                }
                row_height = row_height.max(size.height + theme.row_padding * 2.0);
                (text, truncated)
            })
            .collect();
        Cells {
            texts,
            index_width,
            column_widths: text_widths
                .iter()
                .map(|width| index_width + INDEX_GAP + width)
                .collect(),
            row_height,
        }
    }

    /// `text` 寬度超過 `max_width` 就從末尾去字、補上「…」直到放得下；返回顯示文字與是否截斷過。
    fn truncate(&self, text: &str, font: &NSFont, max_width: f64) -> (String, bool) {
        if text.is_empty() || self.measure(text, font).width <= max_width {
            return (text.to_owned(), false);
        }
        let mut kept: Vec<char> = text.chars().collect();
        while kept.pop().is_some() {
            let mut candidate: String = kept.iter().collect();
            candidate.push_str(ELLIPSIS);
            if kept.is_empty() || self.measure(&candidate, font).width <= max_width {
                return (candidate, true);
            }
        }
        (ELLIPSIS.to_owned(), true)
    }
}
