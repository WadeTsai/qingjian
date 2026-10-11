//! 前臺是全屏應用（遊戲、看影片、演示）時收起狀態條，退出全屏再放出來。
//! 按 F11 進全屏不換前臺視窗、沒有事件可等，狀態條顯示期間每秒看一次。

use core::cell::Cell;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetForegroundWindow, GetShellWindow, GetWindowLongW, GetWindowRect, IsZoomed,
    KillTimer, SW_HIDE, SW_SHOWNA, SetTimer, ShowWindow, WS_CAPTION,
};

/// 狀態條視窗上的定時器編號。
pub(super) const TIMER_ID: usize = 1;

const INTERVAL_MS: u32 = 1000;

/// 顯示狀態條，前臺正全屏就先收著；開始每秒檢查。
pub(super) fn show(hwnd: HWND, hidden: &Cell<bool>) {
    unsafe { SetTimer(Some(hwnd), TIMER_ID, INTERVAL_MS, None) };
    hidden.set(false);
    on_timer(hwnd, hidden);
    if !hidden.get() {
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNA) };
    }
}

/// 收起狀態條（青簡不在前臺 / 關掉了），不再檢查。
pub(super) fn hide(hwnd: HWND, hidden: &Cell<bool>) {
    let _ = unsafe { KillTimer(Some(hwnd), TIMER_ID) };
    hidden.set(false);
    let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
}

/// 每秒一次：進全屏收起，出全屏放回。
pub(super) fn on_timer(hwnd: HWND, hidden: &Cell<bool>) {
    let fullscreen = foreground_is_fullscreen();
    if fullscreen != hidden.get() {
        hidden.set(fullscreen);
        let command = if fullscreen { SW_HIDE } else { SW_SHOWNA };
        let _ = unsafe { ShowWindow(hwnd, command) };
    }
}

/// 前臺視窗蓋滿了它所在的整個顯示器（連工作列一起）。帶標題欄的最大化視窗不算：
/// 工作列自動隱藏時它也蓋滿整屏，但使用者要的是照常顯示。
fn foreground_is_fullscreen() -> bool {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() || hwnd == unsafe { GetShellWindow() } {
        return false;
    }
    let zoomed_with_caption = unsafe { IsZoomed(hwnd) }.as_bool()
        && unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32 & WS_CAPTION.0 == WS_CAPTION.0;
    if zoomed_with_caption {
        return false;
    }
    let mut window = RECT::default();
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        if GetWindowRect(hwnd, &mut window).is_err() {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }
    }
    let screen = info.rcMonitor;
    window.left <= screen.left
        && window.top <= screen.top
        && window.right >= screen.right
        && window.bottom >= screen.bottom
}
