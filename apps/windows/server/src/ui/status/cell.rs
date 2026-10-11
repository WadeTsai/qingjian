//! 狀態條的一格。

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::HFONT;

use super::placement::StatusAction;

/// 一格：文字、字型、顏色、點下去做什麼。
pub(super) struct CellSpec {
    pub(super) text: String,

    pub(super) font: HFONT,

    pub(super) color: COLORREF,

    pub(super) action: StatusAction,
}
