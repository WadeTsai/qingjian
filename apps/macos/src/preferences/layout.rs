use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// 行高。
pub const ROW_HEIGHT: f64 = 24.0;

/// 行距。
pub const ROW_GAP: f64 = 8.0;

/// 頁內左右留白。
pub const PAGE_PADDING: f64 = 20.0;

/// 標題列的寬度（標題右對齊貼著控制元件）。
pub const LABEL_WIDTH: f64 = 110.0;

/// 控制元件列的 x：標題列 + 間隔。
pub const CONTROL_X: f64 = PAGE_PADDING + LABEL_WIDTH + 10.0;

/// 每個標籤頁的內容寬度。
pub const PAGE_WIDTH: f64 = 600.0;

/// 在一頁裡自上而下襬控制元件的簡易佈局：AppKit 座標原點在左下，先按「離頂部多遠」記下來，
/// 最後知道頁高了再一次性換算成 frame。
pub struct Layout {
    /// 這一頁的寬度。
    width: f64,

    /// 已擺的控制元件及其（x, 頂部距離, 寬, 高, 是否撐到頁底）。
    placed: Vec<(Retained<NSView>, f64, f64, f64, f64, bool)>,

    /// 當前行的頂部距離。
    top: f64,
}

impl Layout {
    pub fn new(width: f64, top_margin: f64) -> Self {
        Self {
            width,
            placed: Vec::new(),
            top: top_margin,
        }
    }

    /// 控制元件列從 [`CONTROL_X`] 到右邊留白之間的寬度。
    pub fn control_width(&self) -> f64 {
        self.width - CONTROL_X - PAGE_PADDING
    }

    /// 從左留白到右留白的整行寬度。
    pub fn inner_width(&self) -> f64 {
        self.width - 2.0 * PAGE_PADDING
    }

    /// 在當前行放一個控制元件。
    pub fn place(&mut self, view: &NSView, x: f64, width: f64, height: f64) {
        self.placed
            .push((view.retain(), x, self.top, width, height, false));
    }

    /// 在當前行放一個控制元件，頁高定下來後撐到頁底（至少 `min_height`）；視窗比這一頁高時不留空。
    pub fn place_fill(&mut self, view: &NSView, x: f64, width: f64, min_height: f64) {
        self.placed
            .push((view.retain(), x, self.top, width, min_height, true));
    }

    /// 換到下一行。
    pub fn next_row(&mut self, height: f64) {
        self.top += height + ROW_GAP;
    }

    /// 額外留白（分組之間）。
    pub fn space(&mut self, height: f64) {
        self.top += height;
    }

    /// 到目前為止用掉的高度（含頂部留白）。
    pub fn height(&self) -> f64 {
        self.top
    }

    /// 把所有控制元件加進容器並按容器高度設好 frame（頂部對齊）。
    pub fn finish(self, container: &NSView, total_height: f64) {
        for (view, x, top, width, height, fill) in self.placed {
            let height = if fill {
                (total_height - top - PAGE_PADDING).max(height)
            } else {
                height
            };
            let y = total_height - top - height;
            view.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
            container.addSubview(&view);
        }
    }
}
