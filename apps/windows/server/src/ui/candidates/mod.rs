//! 候選視窗：不搶焦點、置頂的分層視窗，跟隨游標，畫拼音行與候選列表，四周柔和陰影。
//! 預設交給青簡渲染器出點陣圖再貼（[`super::painter`]），配置 `renderer = "system"` 時走 GDI：繪製在 [`view`]，
//! 配色 / 字型在 [`theme`]。繪製內容在 [`RenderData`]，一行的展示形態在 [`row`]。設計語言對齊 macOS 端。

mod click;
mod render_data;
pub(crate) mod row;
pub(crate) mod theme;
pub(crate) mod view;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::Foundation::{E_INVALIDARG, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, IDC_ARROW, KillTimer, LoadCursorW, MSG,
    SW_HIDE, SW_SHOWNA, SetTimer, ShowWindow, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_NCHITTEST,
    WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};
use windows::core::{Error, PCWSTR, Result, w};

use qingjian_platform::Appearance;
use qingjian_platform::protocol::Frame;
use qingjian_render::Mode;

pub use self::click::CandidateClicks;
pub(crate) use self::render_data::RenderData;
use self::theme::Theme;
use super::layered::{self, Layered};
use super::monitor;
use super::painter::SharedPainter;
use super::window_class::WindowClass;

const CLASS_NAME: PCWSTR = w!("QingjianCandidateWindow");
static CLASS: WindowClass = WindowClass::new();

/// 動畫定時器的 id（間隔按渲染器給的 `next_frame`；系統定時器精度約 15.6 ms）。
const ANIMATION_TIMER: usize = 1;

/// 游標行與候選窗之間的間隙（邏輯畫素）。
const CARET_GAP: i32 = 2;

/// 按外觀模式解析深淺；`System` 讀系統主題。
pub(super) fn resolve_dark(mode: Appearance) -> bool {
    match mode {
        Appearance::Light => false,
        Appearance::Dark => true,
        Appearance::System => system_prefers_dark(),
    }
}

/// `HKCU\...\Themes\Personalize\AppsUseLightTheme` 為 0 是深色；讀不到當淺色。
fn system_prefers_dark() -> bool {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_u32("AppsUseLightTheme"))
        .is_ok_and(|value| value == 0)
}

/// 候選視窗。內容經 `UpdateLayeredWindow` 一次貼上，視窗過程只走預設處理。
pub(crate) struct CandidateWindow {
    hwnd: HWND,

    /// 繪製內容。
    data: RefCell<RenderData>,

    /// 上次用的 DPI，變了重建字型。
    dpi: Cell<u32>,

    /// 上次解析出的深淺，變了重建配色。
    dark: Cell<bool>,

    /// 上次記進日誌的縮放值（視窗 DPI、游標所在顯示器 DPI）：變了才再記一條（#146）。
    logged_dpi: Cell<Option<(u32, Option<u32>)>>,

    /// 青簡渲染器；`None` 走 GDI。
    painter: SharedPainter,

    /// 最近一次貼圖時視窗左上角的螢幕座標；動畫幀貼在同一處。
    position: Cell<(i32, i32)>,
}

impl CandidateWindow {
    /// 建一個隱藏的候選視窗；點中候選或譯詞時調 `on_click`。
    pub(crate) fn new(painter: SharedPainter, on_click: CandidateClicks) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::module_handle(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        let dpi = unsafe { GetDpiForSystem() }.max(96);
        let dark = resolve_dark(Appearance::default());
        let data = RefCell::new(RenderData::empty(Rc::new(Theme::new(dpi, dark))));
        // NOACTIVATE：顯示時不搶應用焦點。滑鼠只在候選與譯詞上收，其餘穿透（見 [`click`]）。
        click::install(on_click);
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("青簡候選"),
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
        Ok(Self {
            hwnd,
            data,
            dpi: Cell::new(dpi),
            dark: Cell::new(dark),
            logged_dpi: Cell::new(None),
            painter,
            position: Cell::new((0, 0)),
        })
    }

    /// 重新整理內容（不定位、不顯示）。
    pub(crate) fn set_content(&self, frame: &Frame, mode: Mode) {
        self.data.borrow_mut().set(frame, mode);
    }

    /// 按游標矩形定位並顯示：貼游標下方（放不下放上方），四周留出陰影。
    pub(crate) fn show(&self, anchor: RECT) {
        self.sync_theme(anchor);
        let rendered = {
            let data = self.data.borrow();
            self.painter.borrow_mut().as_mut().and_then(|painter| {
                painter.render_frame(
                    &data.render_frame(),
                    data.layout,
                    self.dark.get(),
                    self.dpi.get(),
                )
            })
        };
        let updated = match rendered {
            Some(rendered) => {
                let content = (
                    rendered.content_width as i32,
                    rendered.content_height as i32,
                );
                if content.0 <= 0 || content.1 <= 0 {
                    self.hide();
                    return;
                }
                let (content_x, content_y) = place(anchor, content);
                let position = (
                    content_x - rendered.content_x as i32,
                    content_y - rendered.content_y as i32,
                );
                self.position.set(position);
                self.schedule(rendered.next_frame);
                click::set_hits(rendered.hits.clone(), (content_x, content_y));
                layered::present(self.hwnd, &rendered.pixmap, position)
            }
            None => {
                click::set_hits(Vec::new(), (0, 0));
                self.show_gdi(anchor)
            }
        };
        if updated.is_ok() {
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNA) };
        } else {
            self.hide();
        }
    }

    /// GDI 畫法：量尺寸、定位、合成。
    fn show_gdi(&self, anchor: RECT) -> Result<()> {
        let margin = layered::shadow_margin(self.dpi.get());
        let content = self.preferred_size();
        if content.0 <= 0 || content.1 <= 0 {
            return Err(Error::from(E_INVALIDARG));
        }
        let (content_x, content_y) = place(anchor, content);
        let data = self.data.borrow();
        layered::composite(
            self.hwnd,
            &Layered {
                content,
                margin,
                win_pos: (content_x - margin, content_y - margin),
                win_size: (content.0 + margin * 2, content.1 + margin * 2),
                background: data.theme.background,
                corner_radius: data.theme.corner_radius,
                paint: &|hdc, client| view::paint(hdc, &data, client),
            },
        )
    }

    pub(crate) fn hide(&self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        click::set_hits(Vec::new(), (0, 0));
        self.stop_animation();
        if let Some(painter) = self.painter.borrow_mut().as_mut() {
            painter.forget();
        }
    }

    /// 這條訊息是候選窗的動畫定時器。
    pub(crate) fn is_animation_timer(&self, msg: &MSG) -> bool {
        msg.message == WM_TIMER && msg.hwnd == self.hwnd && msg.wParam.0 == ANIMATION_TIMER
    }

    /// 定時器每跳：要動畫的下一幀貼在原處；播完就停。
    pub(crate) fn animation_frame(&self) {
        let rendered = self
            .painter
            .borrow_mut()
            .as_mut()
            .and_then(|painter| painter.tick());
        let Some(rendered) = rendered else {
            self.stop_animation();
            return;
        };
        let _ = layered::present(self.hwnd, &rendered.pixmap, self.position.get());
        self.schedule(rendered.next_frame);
    }

    /// 按渲染器給的間隔（過渡 16 ms、迴圈動畫 33 ms）起或改定時器；`None` 停。同一個 id 再 SetTimer 就是改間隔。
    fn schedule(&self, next_frame: Option<std::time::Duration>) {
        match next_frame {
            Some(interval) => {
                let ms = interval.as_millis().clamp(10, 1000) as u32;
                // SAFETY: 本執行緒建的視窗；定時器訊息由 UI 執行緒的訊息迴圈截下交給 animation_frame
                unsafe { SetTimer(Some(self.hwnd), ANIMATION_TIMER, ms, None) };
            }
            None => self.stop_animation(),
        }
    }

    fn stop_animation(&self) {
        // SAFETY: 沒在跳時 KillTimer 返回錯誤，忽略
        let _ = unsafe { KillTimer(Some(self.hwnd), ANIMATION_TIMER) };
    }

    /// DPI 或深淺變了就重建主題；每次 `show` 前調。
    ///
    /// DPI 取游標所在顯示器的：視窗藏著時改了縮放（或睡眠喚醒後多顯示器重排），
    /// `GetDpiForWindow` 會停在舊值，候選字就大小不對（#146）。
    fn sync_theme(&self, anchor: RECT) {
        let caret = POINT {
            x: anchor.left,
            y: anchor.top,
        };
        let monitor_dpi = monitor::dpi_near(caret);
        let window_dpi = unsafe { GetDpiForWindow(self.hwnd) };
        let dpi = match (monitor_dpi, window_dpi) {
            (Some(dpi), _) => dpi,
            (None, 0) => self.dpi.get(),
            (None, dpi) => dpi,
        };
        self.log_dpi(caret, window_dpi, monitor_dpi, dpi);
        let dark = resolve_dark(self.data.borrow().appearance);
        if dpi != self.dpi.get() || dark != self.dark.get() {
            self.data.borrow_mut().theme = Rc::new(Theme::new(dpi, dark));
            self.dpi.set(dpi);
            self.dark.set(dark);
        }
    }

    /// 縮放值變了就記一條，多顯示器 / 睡眠喚醒的問題從日誌裡能看出取到的是哪個值（#146）。
    fn log_dpi(&self, caret: POINT, window_dpi: u32, monitor_dpi: Option<u32>, used: u32) {
        if self.logged_dpi.replace(Some((window_dpi, monitor_dpi)))
            == Some((window_dpi, monitor_dpi))
        {
            return;
        }
        tracing::info!(
            window_dpi,
            ?monitor_dpi,
            used,
            system_dpi = unsafe { GetDpiForSystem() },
            caret_x = caret.x,
            caret_y = caret.y,
            "候選視窗縮放值"
        );
    }

    /// 內容需要的大小（不含陰影留白）。
    fn preferred_size(&self) -> (i32, i32) {
        let hdc = unsafe { GetDC(Some(self.hwnd)) };
        let size = view::preferred_size(hdc, &self.data.borrow());
        unsafe { ReleaseDC(Some(self.hwnd), hdc) };
        (size.cx, size.cy)
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

/// 內容左上角：貼游標下方，放不下放上方，再放不下貼螢幕內；都夾在所在顯示器工作區裡。
fn place(anchor: RECT, content: (i32, i32)) -> (i32, i32) {
    let work = monitor::work_area_near(POINT {
        x: anchor.left,
        y: anchor.top,
    });
    let x = anchor
        .left
        .clamp(work.left, (work.right - content.0).max(work.left));
    let below = anchor.bottom + CARET_GAP;
    let above = anchor.top - CARET_GAP - content.1;
    let y = if below + content.1 <= work.bottom {
        below
    } else if above >= work.top {
        above
    } else {
        (work.bottom - content.1).max(work.top)
    };
    (x, y)
}

/// 分層視窗無需 `WM_PAINT`；滑鼠交給 [`click`]，其餘全交預設處理。
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => click::hit_test(lparam),
        WM_MOUSEACTIVATE => click::mouse_activate(),
        WM_LBUTTONDOWN => {
            click::button_down();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
