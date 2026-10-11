//! 檢視該有的大小，與其中內容區（視窗本體，不含投影留邊）的位置。

use objc2_foundation::{NSRect, NSSize};

#[derive(Debug, Clone, Copy)]
pub struct ViewBounds {
    /// 整個檢視（點），含主題投影留出的邊。
    pub size: NSSize,

    /// 內容區（點），原點是檢視左下角（AppKit 座標），視窗按它貼游標。
    pub content: NSRect,
}

impl ViewBounds {
    /// 沒有留邊：內容區就是整個檢視。
    pub fn filled(size: NSSize) -> Self {
        Self {
            size,
            content: NSRect::new(objc2_foundation::NSPoint::ZERO, size),
        }
    }
}
