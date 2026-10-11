//! 候選視窗的定位錨點：組句範圍 / 選區在螢幕上的矩形，拿不到時退到滑鼠位置。

use std::mem::ManuallyDrop;

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::UI::TextServices::{ITfContext, ITfRange, TF_DEFAULT_SELECTION, TF_SELECTION};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::core::BOOL;

use qingjian_platform::protocol::ScreenRect;

/// `range` 的螢幕矩形，拿不到（有些應用給全零 / 空矩形）退到滑鼠處。
pub(crate) fn anchor_rect(context: &ITfContext, ec: u32, range: &ITfRange) -> ScreenRect {
    to_screen(range_rect(context, ec, range).unwrap_or_else(mouse_anchor))
}

/// 插入點（沒有選區時是游標，有選區時是選區）的螢幕矩形。「只在候選視窗」模式應用裡不放 marked text，
/// 沒有組句範圍可量，就用它給候選視窗定位。
pub(crate) fn caret_rect(context: &ITfContext, ec: u32) -> ScreenRect {
    match selection_range(context, ec) {
        Some(range) => anchor_rect(context, ec, &range),
        None => mouse_screen_rect(),
    }
}

/// 當前選區的範圍；`GetSelection` 移交所有權，由呼叫方釋放。
pub(crate) fn selection_range(context: &ITfContext, ec: u32) -> Option<ITfRange> {
    let mut selection = [TF_SELECTION::default()];
    let mut fetched = 0u32;
    unsafe {
        context
            .GetSelection(ec, TF_DEFAULT_SELECTION, &mut selection, &mut fetched)
            .ok()?;
    }
    if fetched == 0 {
        return None;
    }
    unsafe { ManuallyDrop::take(&mut selection[0].range) }
}

/// 沒有可量的範圍時的錨點：滑鼠處一個零寬、約一行高的矩形。
pub(crate) fn mouse_screen_rect() -> ScreenRect {
    to_screen(mouse_anchor())
}

fn range_rect(context: &ITfContext, ec: u32, range: &ITfRange) -> Option<RECT> {
    let mut rect = RECT::default();
    let mut clipped = BOOL(0);
    unsafe {
        let view = context.GetActiveView().ok()?;
        view.GetTextExt(ec, range, &mut rect, &mut clipped).ok()?;
    }
    (rect.right > rect.left || rect.bottom > rect.top).then_some(rect)
}

fn mouse_anchor() -> RECT {
    let mut point = POINT::default();
    let _ = unsafe { GetCursorPos(&mut point) };
    RECT {
        left: point.x,
        top: point.y,
        right: point.x,
        bottom: point.y + 16,
    }
}

fn to_screen(rect: RECT) -> ScreenRect {
    ScreenRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}
