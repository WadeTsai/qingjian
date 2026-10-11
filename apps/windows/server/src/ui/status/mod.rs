//! 懸浮狀態條：桌面上常駐、可拖動的三格浮窗 `[中 / 英][，。/ ,.][⚙]`。預設由青簡渲染器畫（[`super::painter`]），
//! `renderer = "system"` 時複用分層視窗合成器與候選視窗的 GDI 主題。
//!
//! 按下滑鼠先 `DragDetect`：挪出拖動閾值就交給系統的移動迴圈（`WM_NCLBUTTONDOWN` + `HTCAPTION`），
//! 結束時 `WM_EXITSIZEMOVE` 報新位置；沒挪就是點選，按 x 落進哪格。`WM_MOUSEACTIVATE` 回 `MA_NOACTIVATE` 不搶焦點。
//! 一格的規格在 [`cell`]，擺放與點選在 [`placement`]。

mod cell;
mod fullscreen;
mod placement;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use windows::Win32::Foundation::{
    COLORREF, E_INVALIDARG, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows::Win32::Graphics::Gdi::{GetDC, HDC, ReleaseDC, SetBkMode, TRANSPARENT};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows::Win32::UI::Input::KeyboardAndMouse::{DragDetect, ReleaseCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, HTCAPTION, HTCLIENT, IDC_HAND,
    LoadCursorW, MA_NOACTIVATE, SendMessageW, WM_EXITSIZEMOVE, WM_LBUTTONDOWN, WM_MOUSEACTIVATE,
    WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{Error, PCWSTR, Result, w};

use qingjian_platform::Appearance;
use qingjian_render::StatusCell;

use self::cell::CellSpec;
use self::placement::{Placement, StatusAction};
use super::StatusEvents;
use super::candidates::resolve_dark;
use super::candidates::theme::Theme;
use super::candidates::view;
use super::layered::{self, Layered};
use super::monitor;
use super::painter::SharedPainter;
use super::window_class::WindowClass;
use crate::dispatch::StatusView;

const CLASS_NAME: PCWSTR = w!("QingjianStatusBar");
static CLASS: WindowClass = WindowClass::new();

/// 狀態條與螢幕邊緣的間隙（邏輯畫素）。
const EDGE_GAP: i32 = 8;

thread_local! {
    /// 本執行緒活著的狀態條：HWND → 擺放狀態。視窗過程按 HWND 查，查不到（已析構）就忽略。
    static PLACEMENTS: RefCell<HashMap<isize, Rc<Placement>>> = RefCell::new(HashMap::new());
}

/// 懸浮狀態條視窗。
pub(super) struct StatusBar {
    hwnd: HWND,

    /// 最近一次要顯示的內容；還沒顯示過時為 `None`。
    data: RefCell<Option<StatusView>>,

    /// 按 DPI / 深淺造好的主題（複用候選視窗那套）。
    theme: RefCell<Rc<Theme>>,

    /// 上次用的 DPI，變了重建主題。
    dpi: Cell<u32>,

    /// 上次解析出的深淺，變了重建配色。
    dark: Cell<bool>,

    /// 擺放狀態，與視窗過程共享。
    placement: Rc<Placement>,

    /// 青簡渲染器；`None` 走 GDI。
    painter: SharedPainter,
}

/// 三格從左到右的動作。
const ACTIONS: [StatusAction; 3] = [
    StatusAction::ToggleMode,
    StatusAction::TogglePunctuation,
    StatusAction::OpenSettings,
];

impl StatusBar {
    /// 建一個隱藏的狀態條視窗。
    pub(super) fn new(events: StatusEvents, painter: SharedPainter) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::module_handle(),
            hCursor: unsafe { LoadCursorW(None, IDC_HAND) }.unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        let dpi = unsafe { GetDpiForSystem() }.max(96);
        let dark = resolve_dark(Appearance::default());
        // NOACTIVATE：顯示時不搶應用焦點。
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("青簡狀態條"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(super::module_handle()),
                None,
            )?
        };
        let placement = Rc::new(Placement::new(hwnd, layered::shadow_margin(dpi), events));
        PLACEMENTS.with(|map| map.borrow_mut().insert(hwnd.0 as isize, placement.clone()));
        Ok(Self {
            hwnd,
            data: RefCell::new(None),
            theme: RefCell::new(Rc::new(Theme::new(dpi, dark))),
            dpi: Cell::new(dpi),
            dark: Cell::new(dark),
            placement,
            painter,
        })
    }

    /// 顯示 / 更新：按記住的位置（首次用 `view.anchor`，都沒有就右下角）擺放並重繪。
    pub(super) fn update(&self, view: StatusView) {
        if self.placement.pos.get().is_none() {
            self.placement.pos.set(view.anchor);
        }
        *self.data.borrow_mut() = Some(view);
        self.sync_theme();
        self.render();
    }

    pub(super) fn hide(&self) {
        fullscreen::hide(self.hwnd, &self.placement.fullscreen_hidden);
    }

    /// DPI 或深淺變了就重建主題。DPI 優先取所在位置顯示器的，理由同候選視窗（#146）。
    fn sync_theme(&self) {
        let monitor_dpi = self
            .placement
            .pos
            .get()
            .and_then(|(x, y)| monitor::dpi_near(POINT { x, y }));
        let dpi = match (monitor_dpi, unsafe { GetDpiForWindow(self.hwnd) }) {
            (Some(dpi), _) => dpi,
            (None, 0) => self.dpi.get(),
            (None, dpi) => dpi,
        };
        let mode = self
            .data
            .borrow()
            .as_ref()
            .map(|view| view.appearance)
            .unwrap_or_default();
        let dark = resolve_dark(mode);
        if dpi != self.dpi.get() || dark != self.dark.get() {
            *self.theme.borrow_mut() = Rc::new(Theme::new(dpi, dark));
            self.dpi.set(dpi);
            self.dark.set(dark);
        }
    }

    /// 模式格的文字：中 / 英 / 注，開著雙拼時跟方案名。
    fn mode_text(view: &StatusView) -> String {
        if view.english {
            "英".to_owned()
        } else if view.zhuyin {
            "注".to_owned()
        } else {
            match &view.scheme {
                Some(scheme) => format!("中 · {scheme}"),
                None => "中".to_owned(),
            }
        }
    }

    /// 渲染器要的三格：模式（品牌色）、標點（生效時品牌色，否則灰）、齒輪。
    fn status_cells(view: &StatusView) -> Vec<StatusCell> {
        vec![
            StatusCell::text(Self::mode_text(view), true),
            StatusCell::text(if view.full_width { "，。" } else { ",." }, view.full_width),
            StatusCell::Gear,
        ]
    }

    /// GDI 畫法的三格，順序同 [`ACTIONS`]。
    fn cells(&self, theme: &Theme) -> Vec<CellSpec> {
        let data = self.data.borrow();
        let Some(view) = data.as_ref() else {
            return Vec::new();
        };
        let punctuation_active = view.full_width;
        vec![
            CellSpec {
                text: Self::mode_text(view),
                font: theme.text_font,
                color: theme.cloud_color,
                action: StatusAction::ToggleMode,
            },
            CellSpec {
                text: if punctuation_active { "，。" } else { ",." }.to_owned(),
                font: theme.text_font,
                color: if punctuation_active {
                    theme.cloud_color
                } else {
                    theme.gloss_color
                },
                action: StatusAction::TogglePunctuation,
            },
            CellSpec {
                text: "\u{2699}".to_owned(),
                font: theme.symbol_font,
                color: theme.gloss_color,
                action: StatusAction::OpenSettings,
            },
        ]
    }

    /// 畫好貼上並顯示；順帶記下各格邊界給點選用。渲染器畫不成就走 GDI。
    fn render(&self) {
        let rendered = {
            let data = self.data.borrow();
            let mut painter = self.painter.borrow_mut();
            match (data.as_ref(), painter.as_mut()) {
                (Some(view), Some(painter)) => painter.render_status(
                    &Self::status_cells(view),
                    &view.mode,
                    self.dark.get(),
                    self.dpi.get(),
                ),
                _ => None,
            }
        };
        let updated = match rendered {
            Some(rendered) => {
                let bitmap = &rendered.rendered;
                let content = (bitmap.content_width as i32, bitmap.content_height as i32);
                if content.0 <= 0 || content.1 <= 0 {
                    self.hide();
                    return;
                }
                let margin = bitmap.content_x as i32;
                self.placement.margin.set(margin);
                *self.placement.cells.borrow_mut() = rendered
                    .cell_edges
                    .iter()
                    .zip(ACTIONS)
                    .map(|(edge, action)| (edge.round() as i32, action))
                    .collect();
                let anchor = self.anchor(content, margin);
                layered::present(
                    self.hwnd,
                    &bitmap.pixmap,
                    (anchor.0 - margin, anchor.1 - margin),
                )
            }
            None => self.render_gdi(),
        };
        if updated.is_ok() {
            fullscreen::show(self.hwnd, &self.placement.fullscreen_hidden);
        } else {
            self.hide();
        }
    }

    /// 內容左上角：記住的位置，沒有就右下角，再夾進工作區；順帶記下。
    fn anchor(&self, content: (i32, i32), margin: i32) -> (i32, i32) {
        let anchor = self
            .placement
            .pos
            .get()
            .unwrap_or_else(|| default_anchor(content, margin));
        let anchor = clamp_anchor(anchor, content, margin);
        self.placement.pos.set(Some(anchor));
        anchor
    }

    /// GDI 畫法：量各格、算內容尺寸、擺位置、合成貼上。
    fn render_gdi(&self) -> Result<()> {
        let theme = self.theme.borrow().clone();
        let margin = layered::shadow_margin(self.dpi.get());
        self.placement.margin.set(margin);
        let cells = self.cells(&theme);
        let hdc = unsafe { GetDC(Some(self.hwnd)) };
        let sizes: Vec<SIZE> = cells
            .iter()
            .map(|cell| view::measure(hdc, cell.font, &cell.text))
            .collect();
        unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        let line = sizes.iter().map(|size| size.cy).max().unwrap_or(0);
        // 每格：左右各一個 padding；格間一條細線。
        let widths: Vec<i32> = sizes
            .iter()
            .map(|size| size.cx + theme.padding * 2)
            .collect();
        let content = (widths.iter().sum::<i32>(), line + theme.padding);
        if content.0 <= 0 || content.1 <= 0 || cells.is_empty() {
            return Err(Error::from(E_INVALIDARG));
        }
        let mut right = 0;
        let bounds: Vec<(i32, StatusAction)> = cells
            .iter()
            .zip(&widths)
            .map(|(cell, width)| {
                right += width;
                (right, cell.action)
            })
            .collect();
        *self.placement.cells.borrow_mut() = bounds;
        let anchor = self.anchor(content, margin);

        let separator = theme.pos_color;
        let inset = theme.padding / 2;
        layered::composite(
            self.hwnd,
            &Layered {
                content,
                margin,
                win_pos: (anchor.0 - margin, anchor.1 - margin),
                win_size: (content.0 + margin * 2, content.1 + margin * 2),
                background: theme.background,
                corner_radius: theme.corner_radius,
                paint: &|hdc, client| {
                    unsafe { SetBkMode(hdc, TRANSPARENT) };
                    paint_cells(hdc, client, &cells, &sizes, &widths, separator, inset);
                },
            },
        )
    }
}

/// 每格文字居中；格與格之間一條上下留 `inset` 的細線。
fn paint_cells(
    hdc: HDC,
    client: RECT,
    cells: &[CellSpec],
    sizes: &[SIZE],
    widths: &[i32],
    separator: COLORREF,
    inset: i32,
) {
    let mut x = 0;
    for (index, ((cell, size), width)) in cells.iter().zip(sizes).zip(widths).enumerate() {
        if index > 0 {
            view::fill_rect(
                hdc,
                RECT {
                    left: x,
                    top: inset,
                    right: x + 1,
                    bottom: client.bottom - inset,
                },
                separator,
            );
        }
        let ox = x + (width - size.cx) / 2;
        let oy = (client.bottom - size.cy) / 2;
        view::draw_text(hdc, cell.font, cell.color, ox, oy, &cell.text);
        x += width;
    }
}

impl Drop for StatusBar {
    fn drop(&mut self) {
        PLACEMENTS.with(|map| map.borrow_mut().remove(&(self.hwnd.0 as isize)));
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// 首次出現的位置：主顯示器工作區右下角，留出邊距與陰影。
fn default_anchor(content: (i32, i32), margin: i32) -> (i32, i32) {
    let work = monitor::primary_work_area();
    let gap = ((EDGE_GAP * margin) / 16).max(EDGE_GAP);
    (
        work.right - margin - gap - content.0,
        work.bottom - margin - gap - content.1,
    )
}

/// 把內容左上角夾進所在顯示器的工作區，使整塊內容可見。
fn clamp_anchor(anchor: (i32, i32), content: (i32, i32), margin: i32) -> (i32, i32) {
    let work = monitor::work_area_near(POINT {
        x: anchor.0,
        y: anchor.1,
    });
    let x = anchor.0.clamp(
        work.left + margin,
        (work.right - margin - content.0).max(work.left + margin),
    );
    let y = anchor.1.clamp(
        work.top + margin,
        (work.bottom - margin - content.1).max(work.top + margin),
    );
    (x, y)
}

fn placement_of(hwnd: HWND) -> Option<Rc<Placement>> {
    // clone 出來放開借用，再調回調。
    PLACEMENTS.with(|map| map.borrow().get(&(hwnd.0 as isize)).cloned())
}

/// 按下：拖動交給系統移動迴圈，沒拖就是點選；點選不啟用；拖動結束報位置。
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTCLIENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONDOWN => {
            let mut point = POINT::default();
            let _ = unsafe { GetCursorPos(&mut point) };
            if unsafe { DragDetect(hwnd, point) }.as_bool() {
                let _ = unsafe { ReleaseCapture() };
                unsafe {
                    SendMessageW(
                        hwnd,
                        WM_NCLBUTTONDOWN,
                        Some(WPARAM(HTCAPTION as usize)),
                        Some(LPARAM(0)),
                    )
                };
            } else if let Some(placement) = placement_of(hwnd) {
                // lparam 低 16 位是客戶區 x（有符號）。
                placement.on_click((lparam.0 & 0xFFFF) as i16 as i32);
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == fullscreen::TIMER_ID => {
            if let Some(placement) = placement_of(hwnd) {
                fullscreen::on_timer(hwnd, &placement.fullscreen_hidden);
            }
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            if let Some(placement) = placement_of(hwnd) {
                placement.on_moved();
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
