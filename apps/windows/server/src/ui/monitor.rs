//! 顯示器工作區查詢，候選視窗與狀態條共用。

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY, MONITOR_FROM_FLAGS,
    MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

/// `point` 所在（最近）顯示器的工作區。
pub(super) fn work_area_near(point: POINT) -> RECT {
    work_area(point, MONITOR_DEFAULTTONEAREST)
}

/// 主顯示器的工作區。
pub(super) fn primary_work_area() -> RECT {
    work_area(POINT::default(), MONITOR_DEFAULTTOPRIMARY)
}

/// 拿不到時給個大範圍，至少別把視窗夾沒。
fn work_area(point: POINT, flags: MONITOR_FROM_FLAGS) -> RECT {
    let mut info = MONITORINFO {
        cbSize: core::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let found = unsafe {
        let monitor = MonitorFromPoint(point, flags);
        GetMonitorInfoW(monitor, &mut info).as_bool()
    };
    if found {
        info.rcWork
    } else {
        RECT {
            left: 0,
            top: 0,
            right: i32::MAX,
            bottom: i32::MAX,
        }
    }
}

/// `point` 所在（最近）顯示器的有效 DPI；取不到為 `None`。
pub(super) fn dpi_near(point: POINT) -> Option<u32> {
    let (mut x, mut y) = (0, 0);
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y).ok()?;
    }
    Some(x)
}
