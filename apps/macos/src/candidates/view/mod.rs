//! 候選視窗的內容檢視：自繪頂部拼音行與若干候選，豎排一行一個、橫排排成一行，一項高亮。
//!
//! 有兩條畫法：預設交給 `qingjian-render` 出點陣圖再貼（[`BitmapPainter`]），配置 `[general] renderer = "system"`
//! 走下面用 AppKit 逐項繪製的舊路徑（過渡期的退路，渲染器穩定一個版本後刪）。

mod click;
mod matrix;

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSAttributedStringNSStringDrawing, NSBezierPath, NSColor, NSEvent, NSFont, NSFontAttributeName,
    NSForegroundColorAttributeName, NSStrikethroughStyleAttributeName, NSView,
};
use objc2_foundation::{
    NSArray, NSAttributedString, NSDictionary, NSNumber, NSPoint, NSRect, NSSize, NSString,
};
use qingjian_platform::{CandidateRenderer, LayoutMode};

use super::animation::AnimationTimer;
use super::bitmap::BitmapPainter;
use super::bounds::ViewBounds;
use super::cloud_icon::CloudIcon;
use super::frame::Frame;
use super::preedit::Preedit;
use super::preedit::PreeditStyle;
use super::row::{Row, Tone};
use super::theme::Theme;

pub use click::ClickHandler;

/// 檢視狀態。
pub struct Ivars {
    /// 當前顯示的一幀。
    frame: RefCell<Frame>,

    /// 豎排 / 橫排。
    layout: Cell<LayoutMode>,

    /// 雲聯想的小云朵。
    cloud: CloudIcon,

    /// 主題。
    theme: Theme,

    /// 點陣圖渲染器；`None` 走 AppKit 逐項繪製。按配置建或丟。
    bitmap: RefCell<Option<BitmapPainter>>,

    /// 使用者選的字族名（空為系統字型），換了要重建渲染器。
    font: RefCell<String>,

    /// 渲染器用的主題（淺色那一份），重建渲染器時要帶上。
    render_theme: RefCell<qingjian_render::Theme>,

    /// 動畫定時器：點陣圖渲染器說有動畫在播時跳。
    animation: RefCell<AnimationTimer>,

    /// 點中候選或譯詞時交給誰（host 註冊）。
    on_click: RefCell<Option<ClickHandler>>,
}

/// preedit 游標的寬度。
const CARET_WIDTH: f64 = 1.5;

/// 雲朵圖示邊長。
const CLOUD_SIZE: f64 = 13.0;

/// 雲朵與後面文字的間距。
const CLOUD_GAP: f64 = 4.0;

/// 拿不到 SF Symbol 時的文字雲朵。
const CLOUD_FALLBACK: &str = "☁︎";

/// preedit 與右側整句補全之間的間距。
const SENTENCE_GAP: f64 = 16.0;

/// 橫排時序號與候選詞之間的間距。
const INDEX_GAP: f64 = 3.0;

/// 橫排時高亮底色在候選兩側多出的寬度。
const HIGHLIGHT_INSET: f64 = 5.0;

/// `NSUnderlineStyleSingle`：刪除線用單線。
const STRIKE_SINGLE: isize = 1;

/// 豎排的列寬與行高。
struct Columns {
    index_width: f64,
    text_width: f64,
    annotation_width: f64,
    row_height: f64,
}

/// 橫排時每一項的尺寸。
struct Item {
    index_width: f64,
    text_width: f64,
}

define_class!(
    // SAFETY: NSView 允許子類化；沒有實現 Drop。
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    pub struct CandidateView;

    impl CandidateView {
        /// 用左上角為原點的座標系，行從上往下畫。
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        /// 面板不啟用，第一下點選就要算數（否則先被當成「啟用視窗」吃掉）。
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.clicked(event);
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            if let Some(bitmap) = &mut *self.ivars().bitmap.borrow_mut() {
                bitmap.draw(self.is_dark(), self.backing_scale());
            } else {
                self.draw();
            }
        }
    }
);

impl CandidateView {
    pub fn new(mtm: MainThreadMarker, theme: Theme) -> Retained<Self> {
        let cloud = CloudIcon::new(&theme.cloud_color, CLOUD_SIZE);
        let this = mtm.alloc::<Self>().set_ivars(Ivars {
            frame: RefCell::new(Frame::default()),
            layout: Cell::new(LayoutMode::default()),
            cloud,
            theme,
            bitmap: RefCell::new(None),
            font: RefCell::new(String::new()),
            render_theme: RefCell::new(qingjian_render::Theme::light()),
            animation: RefCell::new(AnimationTimer::default()),
            on_click: RefCell::new(None),
        });
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }

    /// 候選窗字型（字族名，空為系統字型）。渲染器在用就當場重建。
    pub fn set_font(&self, font: &str) {
        if *self.ivars().font.borrow() == font {
            return;
        }
        *self.ivars().font.borrow_mut() = font.to_owned();
        let mut bitmap = self.ivars().bitmap.borrow_mut();
        if bitmap.is_some() {
            *bitmap = self.new_bitmap();
            drop(bitmap);
            self.setNeedsDisplay(true);
        }
    }

    /// 渲染器主題，只對青簡渲染器生效。配置或主題檔案變了才會調，每次都當場重畫。
    pub fn set_render_theme(&self, theme: qingjian_render::Theme) {
        *self.ivars().render_theme.borrow_mut() = theme.clone();
        if let Some(bitmap) = &mut *self.ivars().bitmap.borrow_mut() {
            bitmap.set_theme(theme);
            self.setNeedsDisplay(true);
        }
        self.sync_animation();
    }

    /// 新建點陣圖渲染器並帶上當前主題。
    fn new_bitmap(&self) -> Option<BitmapPainter> {
        let mut bitmap = BitmapPainter::new(&self.ivars().font.borrow())?;
        bitmap.set_theme(self.ivars().render_theme.borrow().clone());
        Some(bitmap)
    }

    /// 青簡渲染器 / 系統繪製。渲染器字型庫載入失敗就留在系統繪製。
    pub fn set_renderer(&self, renderer: CandidateRenderer) {
        let mut bitmap = self.ivars().bitmap.borrow_mut();
        match renderer {
            CandidateRenderer::Qingjian if bitmap.is_none() => {
                *bitmap = self.new_bitmap();
            }
            CandidateRenderer::System if bitmap.is_some() => {
                tracing::info!("候选窗切回 AppKit 绘制");
                *bitmap = None;
            }
            _ => return,
        }
        drop(bitmap);
        self.setNeedsDisplay(true);
    }

    /// 當前生效的外觀是不是深色。
    fn is_dark(&self) -> bool {
        // SAFETY: 只讀 AppKit 匯出的常量名
        let names =
            unsafe { NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]) };
        let matched = self
            .effectiveAppearance()
            .bestMatchFromAppearancesWithNames(&names);
        // SAFETY: 同上
        matched.is_some_and(|name| unsafe { name.isEqualToString(NSAppearanceNameDarkAqua) })
    }

    /// 所在螢幕的倍數；還沒進視窗時按 Retina 算。
    fn backing_scale(&self) -> f32 {
        self.window()
            .map_or(2.0, |window| window.backingScaleFactor() as f32)
    }

    pub fn theme(&self) -> &Theme {
        &self.ivars().theme
    }

    pub fn set_layout(&self, layout: LayoutMode) {
        self.ivars().layout.set(layout);
    }

    /// 更新內容並返回檢視該有的尺寸與內容區。
    pub fn set_frame(&self, frame: &Frame) -> ViewBounds {
        *self.ivars().frame.borrow_mut() = frame.clone();
        self.setNeedsDisplay(true);
        let bounds = self.ivars().bitmap.borrow_mut().as_mut().map(|bitmap| {
            bitmap.set_frame(
                frame,
                self.ivars().layout.get(),
                self.is_dark(),
                self.backing_scale(),
            )
        });
        match bounds {
            Some(bounds) => {
                self.sync_animation();
                bounds
            }
            None => ViewBounds::filled(self.preferred_size()),
        }
    }

    /// 點陣圖渲染器說有動畫在播就起定時器。
    fn sync_animation(&self) {
        let next_frame = self
            .ivars()
            .bitmap
            .borrow()
            .as_ref()
            .and_then(BitmapPainter::next_frame);
        if let (Some(interval), Some(mtm)) = (next_frame, MainThreadMarker::new()) {
            self.ivars()
                .animation
                .borrow_mut()
                .start(mtm, self, interval);
        }
    }

    /// 定時器每跳：要動畫的下一幀並重畫；播完就停。
    pub fn animation_frame(&self) {
        let next_frame = self
            .ivars()
            .bitmap
            .borrow_mut()
            .as_mut()
            .and_then(BitmapPainter::tick);
        self.setNeedsDisplay(true);
        match (next_frame, MainThreadMarker::new()) {
            // 過渡播完只剩迴圈動畫時，間隔從 16 ms 換成 33 ms
            (Some(interval), Some(mtm)) => self
                .ivars()
                .animation
                .borrow_mut()
                .start(mtm, self, interval),
            _ => self.ivars().animation.borrow_mut().stop(),
        }
    }

    /// 視窗收起：停動畫、忘掉上一幀。
    pub fn stop_animation(&self) {
        self.ivars().animation.borrow_mut().stop();
        if let Some(bitmap) = &mut *self.ivars().bitmap.borrow_mut() {
            bitmap.forget();
        }
    }

    /// 在用青簡渲染器（陰影畫在位圖裡）；否則是 AppKit 逐項繪製，要系統陰影。
    pub fn uses_bitmap(&self) -> bool {
        self.ivars().bitmap.borrow().is_some()
    }

    fn preferred_size(&self) -> NSSize {
        let theme = self.theme();
        let frame = self.ivars().frame.borrow();
        let (top_width, top_height) = self.top_line_size(&frame);
        let (body_width, body_height) = match self.ivars().layout.get() {
            LayoutMode::Vertical => self.vertical_size(&frame),
            LayoutMode::Horizontal if frame.columns > 0 => self.matrix_size(&frame),
            LayoutMode::Horizontal => self.horizontal_size(&frame),
        };
        let width = top_width.max(body_width);
        NSSize::new(
            width + theme.padding * 2.0,
            top_height + body_height + theme.padding * 2.0,
        )
    }

    /// 頂部拼音行（含右側整句補全）需要的寬高；沒有這一行時都是 0。
    fn top_line_size(&self, frame: &Frame) -> (f64, f64) {
        if !frame.has_top_line() {
            return (0.0, 0.0);
        }
        let theme = self.theme();
        let line_height = self.measure("x", &theme.annotation_font).height;
        let mut width = 0.0;
        if let Some(preedit) = &frame.preedit {
            width += self.measure(&preedit.text(), &theme.annotation_font).width + CARET_WIDTH;
        }
        if let Some((text, cloud)) = frame.trailing() {
            if frame.preedit.is_some() {
                width += SENTENCE_GAP;
            }
            if cloud {
                width += self.cloud_width();
            }
            width += self.measure(text, &theme.annotation_font).width;
        }
        (width, line_height + theme.row_padding * 2.0)
    }

    fn vertical_size(&self, frame: &Frame) -> (f64, f64) {
        let theme = self.theme();
        let columns = self.columns(&frame.rows);
        let mut width = columns.index_width + theme.column_gap + columns.text_width;
        if columns.annotation_width > 0.0 {
            width += theme.column_gap + columns.annotation_width;
        }
        let mut height = columns.row_height * frame.rows.len() as f64;
        if let Some(footer) = frame.footer.as_deref() {
            let footer_size = self.measure(footer, &theme.index_font);
            width = width.max(footer_size.width);
            height += footer_size.height + theme.row_padding;
        }
        (width, height)
    }

    fn horizontal_size(&self, frame: &Frame) -> (f64, f64) {
        let theme = self.theme();
        if frame.rows.is_empty() {
            return (0.0, 0.0);
        }
        let (items, row_height) = self.items(&frame.rows);
        let mut width: f64 = items
            .iter()
            .map(|item| item.index_width + INDEX_GAP + item.text_width)
            .sum::<f64>()
            + theme.column_gap * (items.len().saturating_sub(1)) as f64
            + HIGHLIGHT_INSET * 2.0;
        if let Some(footer) = frame.footer.as_deref() {
            width += theme.column_gap + self.measure(footer, &theme.index_font).width;
        }
        let mut height = row_height;
        if let Some((annotation_width, annotation_height)) = self.highlighted_annotation_size(frame)
        {
            width = width.max(annotation_width);
            height += annotation_height;
        }
        (width, height)
    }

    /// 橫排時高亮候選的譯文行尺寸；高亮候選沒有譯文時為 `None`。
    fn highlighted_annotation_size(&self, frame: &Frame) -> Option<(f64, f64)> {
        let theme = self.theme();
        let row = frame.rows.get(frame.highlighted)?;
        if row.annotation.is_empty() {
            return None;
        }
        let width: f64 = row
            .annotation
            .iter()
            .map(|(s, _)| self.measure(s, &theme.annotation_font).width)
            .sum();
        let height = self.measure("x", &theme.annotation_font).height + theme.row_padding;
        Some((width, height))
    }

    /// 雲朵圖標占的寬度（含後面的間距）。
    fn cloud_width(&self) -> f64 {
        let cloud = &self.ivars().cloud;
        let width = if cloud.is_symbol() {
            cloud.width()
        } else {
            self.measure(CLOUD_FALLBACK, &self.theme().annotation_font)
                .width
        };
        width + CLOUD_GAP
    }

    /// 畫雲朵，返回佔用寬度（含間距）。`top` 是所在行文字的頂邊，`line_height` 用來垂直居中。
    fn draw_cloud(&self, x: f64, top: f64, line_height: f64) -> f64 {
        let cloud = &self.ivars().cloud;
        if cloud.is_symbol() {
            cloud.draw(x, top + (line_height - cloud.width()) / 2.0);
        } else {
            let theme = self.theme();
            let size = self.measure(CLOUD_FALLBACK, &theme.annotation_font);
            self.draw_text(
                CLOUD_FALLBACK,
                &theme.annotation_font,
                &theme.cloud_color,
                top + (line_height - size.height) / 2.0,
                x,
            );
        }
        self.cloud_width()
    }

    fn columns(&self, rows: &[Row]) -> Columns {
        let theme = self.theme();
        let mut columns = Columns {
            index_width: 0.0,
            text_width: 0.0,
            annotation_width: 0.0,
            row_height: 0.0,
        };
        for row in rows {
            let index = self.measure(&row.index, &theme.index_font);
            let mut text = self.measure(&row.text, &theme.text_font);
            if row.cloud {
                text.width += self.cloud_width();
            }
            let annotation: f64 = row
                .annotation
                .iter()
                .map(|(s, _)| self.measure(s, &theme.annotation_font).width)
                .sum();
            columns.index_width = columns.index_width.max(index.width);
            columns.text_width = columns.text_width.max(text.width);
            columns.annotation_width = columns.annotation_width.max(annotation);
            columns.row_height = columns
                .row_height
                .max(text.height + theme.row_padding * 2.0);
        }
        columns
    }

    /// 橫排各項的尺寸與統一行高。
    fn items(&self, rows: &[Row]) -> (Vec<Item>, f64) {
        let theme = self.theme();
        let mut row_height: f64 = 0.0;
        let items = rows
            .iter()
            .map(|row| {
                let index = self.measure(&row.index, &theme.index_font);
                let mut text = self.measure(&row.text, &theme.text_font);
                if row.cloud {
                    text.width += self.cloud_width();
                }
                row_height = row_height.max(text.height + theme.row_padding * 2.0);
                Item {
                    index_width: index.width,
                    text_width: text.width,
                }
            })
            .collect();
        (items, row_height)
    }

    fn draw(&self) {
        let theme = self.theme();
        let frame = self.ivars().frame.borrow();
        let bounds = self.bounds();

        // 背景
        theme.background.set();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
            bounds,
            theme.corner_radius,
            theme.corner_radius,
        )
        .fill();

        let mut y = theme.padding;
        y += self.draw_top_line(&frame, y);
        match self.ivars().layout.get() {
            LayoutMode::Vertical => self.draw_vertical(&frame, y, bounds),
            LayoutMode::Horizontal if frame.columns > 0 => self.draw_matrix(&frame, y, bounds),
            LayoutMode::Horizontal => self.draw_horizontal(&frame, y, bounds),
        }
    }

    /// 頂部拼音行：各段按樣式畫、我們自己畫游標（不依賴應用畫插入點）、右側整句補全。返回佔用高度。
    fn draw_top_line(&self, frame: &Frame, y: f64) -> f64 {
        if !frame.has_top_line() {
            return 0.0;
        }
        let theme = self.theme();
        let line_height = self.measure("x", &theme.annotation_font).height;
        let top = y + theme.row_padding;
        let mut x = theme.padding;
        if let Some(preedit) = &frame.preedit {
            x += self.draw_preedit(preedit, x, top, line_height);
            if frame.trailing().is_some() {
                x += SENTENCE_GAP;
            }
        }
        // 整句補全：雲朵 + 句子，顏色與本地候選區分；臨時狀態灰字、不帶雲朵
        if let Some((text, cloud)) = frame.trailing() {
            let color = if cloud {
                x += self.draw_cloud(x, top, line_height);
                &theme.cloud_color
            } else {
                &theme.gloss_color
            };
            self.draw_text(text, &theme.annotation_font, color, top, x);
        }
        line_height + theme.row_padding * 2.0
    }

    /// 畫拼音行的各段與游標，返回佔用寬度（含游標）。
    fn draw_preedit(&self, preedit: &Preedit, x: f64, top: f64, line_height: f64) -> f64 {
        let theme = self.theme();
        let mut cursor_x = x;
        for segment in &preedit.segments {
            let (color, strike) = match segment.style {
                PreeditStyle::Typed => (&theme.gloss_color, false),
                PreeditStyle::Rest => (&theme.pos_color, false),
                PreeditStyle::Struck => (&theme.pos_color, true),
            };
            let string = self.attributed(&segment.text, &theme.annotation_font, color, strike);
            string.drawAtPoint(NSPoint::new(cursor_x, top));
            cursor_x += string.size().width;
        }
        let caret_x = x + self
            .measure(&preedit.before_cursor(), &theme.annotation_font)
            .width;
        theme.text_color.set();
        NSBezierPath::fillRect(NSRect::new(
            NSPoint::new(caret_x, top),
            NSSize::new(CARET_WIDTH, line_height),
        ));
        cursor_x - x + CARET_WIDTH
    }

    fn draw_vertical(&self, frame: &Frame, mut y: f64, bounds: NSRect) {
        let theme = self.theme();
        let columns = self.columns(&frame.rows);
        let text_x = theme.padding + columns.index_width + theme.column_gap;
        let annotation_x = text_x + columns.text_width + theme.column_gap;
        for (i, row) in frame.rows.iter().enumerate() {
            if i == frame.highlighted {
                let rect = NSRect::new(
                    NSPoint::new(theme.padding / 2.0, y),
                    NSSize::new(bounds.size.width - theme.padding, columns.row_height),
                );
                self.fill_highlight(rect);
            }
            // 各列底部對齊到候選詞基線附近：小字往下挪一點
            let text_size = self.measure(&row.text, &theme.text_font);
            let baseline = y + theme.row_padding;
            let small_offset = self.small_offset(text_size.height);
            self.draw_text(
                &row.index,
                &theme.index_font,
                &theme.index_color,
                baseline + small_offset,
                theme.padding,
            );
            self.draw_word(row, text_x, baseline, text_size.height);
            let mut x = annotation_x;
            for (segment, tone) in &row.annotation {
                x += self.draw_text(
                    segment,
                    &theme.annotation_font,
                    self.tone_color(*tone),
                    baseline + small_offset,
                    x,
                );
            }
            y += columns.row_height;
        }
        if let Some(footer) = frame.footer.as_deref() {
            let size = self.measure(footer, &theme.index_font);
            self.draw_text(
                footer,
                &theme.index_font,
                &theme.index_color,
                y + theme.row_padding,
                bounds.size.width - theme.padding - size.width,
            );
        }
    }

    /// 橫排：候選排成一行，高亮那個下面單獨一行譯文，頁碼在行尾。
    fn draw_horizontal(&self, frame: &Frame, y: f64, bounds: NSRect) {
        if frame.rows.is_empty() {
            return;
        }
        let theme = self.theme();
        let (items, row_height) = self.items(&frame.rows);
        let baseline = y + theme.row_padding;
        let mut x = theme.padding + HIGHLIGHT_INSET;
        for (i, (row, item)) in frame.rows.iter().zip(&items).enumerate() {
            let item_width = item.index_width + INDEX_GAP + item.text_width;
            if i == frame.highlighted {
                let rect = NSRect::new(
                    NSPoint::new(x - HIGHLIGHT_INSET, y),
                    NSSize::new(item_width + HIGHLIGHT_INSET * 2.0, row_height),
                );
                self.fill_highlight(rect);
            }
            let text_size = self.measure(&row.text, &theme.text_font);
            self.draw_text(
                &row.index,
                &theme.index_font,
                &theme.index_color,
                baseline + self.small_offset(text_size.height),
                x,
            );
            self.draw_word(
                row,
                x + item.index_width + INDEX_GAP,
                baseline,
                text_size.height,
            );
            x += item_width + theme.column_gap;
        }
        if let Some(footer) = frame.footer.as_deref() {
            let size = self.measure(footer, &theme.index_font);
            let text_height = self.measure("x", &theme.text_font).height;
            self.draw_text(
                footer,
                &theme.index_font,
                &theme.index_color,
                baseline + self.small_offset(text_height),
                bounds.size.width - theme.padding - size.width,
            );
        }
        // 高亮候選的譯文
        if let Some(row) = frame.rows.get(frame.highlighted) {
            let mut x = theme.padding + HIGHLIGHT_INSET;
            let top = y + row_height + theme.row_padding / 2.0;
            for (segment, tone) in &row.annotation {
                x += self.draw_text(
                    segment,
                    &theme.annotation_font,
                    self.tone_color(*tone),
                    top,
                    x,
                );
            }
        }
    }

    /// 候選詞本體：雲端詞前帶雲朵、換顏色。
    fn draw_word(&self, row: &Row, x: f64, baseline: f64, text_height: f64) {
        let theme = self.theme();
        let mut word_x = x;
        if row.cloud {
            word_x += self.draw_cloud(word_x, baseline, text_height);
        }
        let color = if row.cloud {
            &theme.cloud_color
        } else {
            &theme.text_color
        };
        self.draw_text(&row.text, &theme.text_font, color, baseline, word_x);
    }

    fn fill_highlight(&self, rect: NSRect) {
        let theme = self.theme();
        theme.highlight.set();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
            rect,
            theme.corner_radius / 2.0,
            theme.corner_radius / 2.0,
        )
        .fill();
    }

    /// 小字相對候選詞往下挪多少，讓兩者底部對齊。
    fn small_offset(&self, text_height: f64) -> f64 {
        (text_height - self.measure("x", &self.theme().annotation_font).height).max(0.0)
    }

    fn tone_color(&self, tone: Tone) -> &NSColor {
        match tone {
            Tone::Gloss => &self.theme().gloss_color,
            Tone::Fresh => &self.theme().fresh_color,
            Tone::Faint | Tone::Pos | Tone::Separator => &self.theme().pos_color,
        }
    }

    /// 畫一段文字，返回它的寬度。引數順序是 (頂邊 y, 左邊 x)，與畫圖時「先定行再定列」的習慣一致。
    fn draw_text(&self, text: &str, font: &NSFont, color: &NSColor, y: f64, x: f64) -> f64 {
        let string = self.attributed(text, font, color, false);
        string.drawAtPoint(NSPoint::new(x, y));
        string.size().width
    }

    fn measure(&self, text: &str, font: &NSFont) -> NSSize {
        self.attributed(text, font, &self.theme().text_color, false)
            .size()
    }

    fn attributed(
        &self,
        text: &str,
        font: &NSFont,
        color: &NSColor,
        strike: bool,
    ) -> Retained<NSAttributedString> {
        let strike_style = NSNumber::new_isize(STRIKE_SINGLE);
        // SAFETY: 只讀 AppKit 匯出的屬性名常量
        let (keys, objects): (Vec<&NSString>, Vec<&AnyObject>) = unsafe {
            if strike {
                (
                    vec![
                        NSFontAttributeName,
                        NSForegroundColorAttributeName,
                        NSStrikethroughStyleAttributeName,
                    ],
                    vec![font, color, &strike_style],
                )
            } else {
                (
                    vec![NSFontAttributeName, NSForegroundColorAttributeName],
                    vec![font, color],
                )
            }
        };
        let attributes = NSDictionary::from_slices(&keys, &objects);
        unsafe { NSAttributedString::new_with_attributes(&NSString::from_str(text), &attributes) }
    }
}
