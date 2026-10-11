//! 候選視窗：非啟用的浮動 NSPanel，跟隨游標，內容由 [`CandidateView`] 繪製。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSBackingStoreType, NSColor, NSEvent, NSPanel, NSScreen, NSWindowCollectionBehavior,
    NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use qingjian_platform::{Appearance, CandidateRenderer, LayoutMode};

use super::frame::Frame;
use super::theme::Theme;
use super::view::{CandidateView, ClickHandler};

/// `kCGPopUpMenuWindowLevel`：浮在普通視窗和浮動面板之上，與系統輸入法候選框同級。
const POPUP_MENU_LEVEL: NSWindowLevel = 101;

/// 候選視窗與游標行之間的間隙。
const CARET_GAP: f64 = 4.0;

/// 應用給不出游標位置時，拿滑鼠位置當游標，按這個高度算一行。
const FALLBACK_LINE_HEIGHT: f64 = 16.0;

pub struct CandidateWindow {
    /// 面板本體。不在當前 Space 時會整個換新（見 [`Self::order_front_on_active_space`]）。
    panel: Retained<NSPanel>,

    /// 內容檢視；換面板時搬過去。
    view: Retained<CandidateView>,

    /// 當前外觀（跟隨系統時為 `None`）；換面板時要重設。
    appearance: Option<Retained<NSAppearance>>,

    /// 用來取螢幕尺寸。
    mtm: MainThreadMarker,
}

impl CandidateWindow {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let view = CandidateView::new(mtm, Theme::system_default());
        let panel = build_panel(mtm, &view);
        Self {
            panel,
            view,
            appearance: None,
            mtm,
        }
    }

    /// 顯示一幀。`anchor` 是游標行的螢幕矩形，視窗貼在它下方，放不下就放上方。
    pub fn show(&mut self, frame: Frame, anchor: NSRect) {
        if frame.is_empty() {
            self.hide();
            return;
        }
        let bounds = self.view.set_frame(&frame);
        let content = self.place(bounds.content.size, anchor);
        let origin = NSPoint::new(
            content.x - bounds.content.origin.x,
            content.y - bounds.content.origin.y,
        );
        self.panel
            .setFrame_display(NSRect::new(origin, bounds.size), true);
        self.order_front_on_active_space();
        if !self.panel.isVisible() {
            tracing::warn!(?anchor, ?origin, "候选窗口 orderFront 之后仍不可见");
        } else {
            tracing::debug!(
                ?anchor,
                ?origin,
                on_active_space = self.panel.isOnActiveSpace(),
                "候选窗口已显示"
            );
        }
    }

    pub fn hide(&self) {
        self.panel.orderOut(None);
        self.view.stop_animation();
    }

    /// 排到最前，並確認真在當前 Space 上；不在就換一塊新面板。
    ///
    /// collection behavior 是「所有 Space + 全屏輔助」，可 macOS 26 上 WindowServer 只把面板綁到它建立時已有的 Space：
    /// 之後新開的全屏 Space（Zed 全屏就是一個）裡沒有它，候選框留在桌面那個 Space。用 CGS 查過，面板的 Space 列表只有桌面，
    /// 而同樣設定新建的面板會綁到全部 Space；重設 collection behavior、收起再排前都不會讓它重算，只有新建有用。
    /// `isOnActiveSpace` 能反映這個狀態，所以每次顯示後查一下，不在就把內容檢視搬到新面板上。
    fn order_front_on_active_space(&mut self) {
        self.panel.orderFrontRegardless();
        if self.panel.isOnActiveSpace() {
            return;
        }
        let frame = self.panel.frame();
        self.panel.orderOut(None);
        let panel = build_panel(self.mtm, &self.view);
        panel.setAppearance(self.appearance.as_deref());
        panel.setHasShadow(!self.view.uses_bitmap());
        panel.setIgnoresMouseEvents(!self.view.uses_bitmap());
        panel.setFrame_display(frame, true);
        panel.orderFrontRegardless();
        self.panel = panel;
        tracing::warn!(
            recovered = self.panel.isOnActiveSpace(),
            "候选窗口不在当前 Space，已换新面板"
        );
    }

    /// 外觀：跟隨系統時不指定，否則強制淺色 / 深色。
    pub fn set_appearance(&mut self, mode: Appearance) {
        // SAFETY: 只讀 AppKit 匯出的常量名
        let name = unsafe {
            match mode {
                Appearance::System => None,
                Appearance::Light => Some(NSAppearanceNameAqua),
                Appearance::Dark => Some(NSAppearanceNameDarkAqua),
            }
        };
        let appearance = name.and_then(NSAppearance::appearanceNamed);
        self.panel.setAppearance(appearance.as_deref());
        self.appearance = appearance;
    }

    /// 豎排 / 橫排。下一幀生效。
    pub fn set_layout(&self, layout: LayoutMode) {
        self.view.set_layout(layout);
    }

    /// 青簡渲染器 / 系統繪製。下一幀生效。青簡渲染器的陰影由主題畫進點陣圖，系統陰影關掉；
    /// 只有它給點選區域，所以也只有它收滑鼠（系統繪製時點選穿過去）。
    pub fn set_renderer(&self, renderer: CandidateRenderer) {
        self.view.set_renderer(renderer);
        self.panel.setHasShadow(!self.view.uses_bitmap());
        self.panel.setIgnoresMouseEvents(!self.view.uses_bitmap());
    }

    /// 點中候選或譯詞時調誰。
    pub fn set_click_handler(&self, handler: ClickHandler) {
        self.view.set_click_handler(handler);
    }

    /// 在用青簡渲染器（而不是系統繪製）。
    pub fn uses_bitmap(&self) -> bool {
        self.view.uses_bitmap()
    }

    /// 渲染器主題（按 `[general] theme` 從主題庫取出的），只對青簡渲染器生效。
    pub fn set_render_theme(&self, theme: qingjian_render::Theme) {
        self.view.set_render_theme(theme);
    }

    /// 候選窗字型（字族名，空為系統字型），只對青簡渲染器生效。
    pub fn set_font(&self, font: &str) {
        self.view.set_font(font);
    }

    pub fn max_rows(&self) -> usize {
        self.view.theme().max_rows
    }

    /// 內容區左下角座標：貼在游標行下方；下方放不下放上方；不出游標所在的那塊螢幕。
    /// 游標矩形是零或落在所有螢幕之外（應用不支援、或給的是胡話）時以滑鼠位置為準，至少落在使用者看著的螢幕上。
    fn place(&self, size: NSSize, anchor: NSRect) -> NSPoint {
        let (anchor, screen) = match screen_containing(self.mtm, anchor.origin) {
            Some(screen) if !(anchor.size.height == 0.0 && anchor.origin == NSPoint::ZERO) => {
                (anchor, screen)
            }
            _ => {
                let mouse = NSEvent::mouseLocation();
                let anchor = NSRect::new(mouse, NSSize::new(0.0, FALLBACK_LINE_HEIGHT));
                (
                    anchor,
                    screen_containing(self.mtm, mouse)
                        .unwrap_or_else(|| main_screen_or_anywhere(self.mtm)),
                )
            }
        };
        let min_x = screen.origin.x;
        let max_x = (screen.origin.x + screen.size.width - size.width).max(min_x);
        let x = anchor.origin.x.clamp(min_x, max_x);
        let below = anchor.origin.y - CARET_GAP - size.height;
        let above = anchor.origin.y + anchor.size.height + CARET_GAP;
        let top = screen.origin.y + screen.size.height;
        let y = if below >= screen.origin.y {
            below
        } else if above + size.height <= top {
            above
        } else {
            // 上下都放不下（螢幕很矮或視窗很高）：貼螢幕底邊，寧可蓋住游標也別出屏
            screen.origin.y
        };
        // 無論怎麼算，最後都要落在這塊螢幕裡：出屏等於不顯示
        let max_y = (top - size.height).max(screen.origin.y);
        NSPoint::new(x, y.clamp(screen.origin.y, max_y))
    }
}

/// 建一塊麵板並把內容檢視裝進去：無邊框、不搶焦點、透明背景帶系統陰影（用青簡渲染器時再關）、
/// 先不吃滑鼠（用青簡渲染器時再開，見 [`CandidateWindow::set_renderer`]）。
fn build_panel(mtm: MainThreadMarker, view: &CandidateView) -> Retained<NSPanel> {
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        mtm.alloc::<NSPanel>(),
        NSRect::new(NSPoint::ZERO, NSSize::new(200.0, 100.0)),
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    panel.setHasShadow(true);
    panel.setBecomesKeyOnlyIfNeeded(true);
    panel.setIgnoresMouseEvents(true);
    // NSPanel 預設在應用失活時自動隱藏；輸入法程序從來不是前臺應用，不能靠這個
    panel.setHidesOnDeactivate(false);
    panel.setCollectionBehavior(collection_behavior());
    // 不要 setFloatingPanel(true)：它會把層級改回 NSFloatingWindowLevel（3），全屏應用的 Space 裡就看不見了；
    // 層級最後設，別被前面任何一項覆蓋
    panel.setLevel(POPUP_MENU_LEVEL);
    panel.setContentView(Some(view));
    panel
}

/// 面板的 Space 歸屬：出現在所有 Space 上、能與全屏應用同處一個 Space、Mission Control 裡不動。
fn collection_behavior() -> NSWindowCollectionBehavior {
    NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Stationary
}

/// 包含 `point` 的那塊螢幕的可見區域；哪塊都不包含返回 `None`。
fn screen_containing(mtm: MainThreadMarker, point: NSPoint) -> Option<NSRect> {
    let screens = NSScreen::screens(mtm);
    let hit = screens.iter().find(|screen| {
        let frame = screen.frame();
        point.x >= frame.origin.x
            && point.x < frame.origin.x + frame.size.width
            && point.y >= frame.origin.y
            && point.y < frame.origin.y + frame.size.height
    });
    hit.map(|screen| screen.visibleFrame())
}

/// 主屏的可見區域；連主屏都沒有時不限制。
fn main_screen_or_anywhere(mtm: MainThreadMarker) -> NSRect {
    NSScreen::mainScreen(mtm)
        .map(|s| s.visibleFrame())
        .unwrap_or_else(|| {
            NSRect::new(
                NSPoint::new(f64::MIN / 2.0, f64::MIN / 2.0),
                NSSize::new(f64::MAX, f64::MAX),
            )
        })
}
