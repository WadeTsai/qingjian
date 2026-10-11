use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{E_FAIL, ERROR_CLASS_ALREADY_EXISTS, GetLastError};
use windows::Win32::UI::WindowsAndMessaging::{RegisterClassExW, WNDCLASSEXW};
use windows::core::{Error, Result};

/// 程序內只註冊一次的視窗類，放 `static` 裡，建視窗前 [`Self::ensure`]。
pub(crate) struct WindowClass {
    /// 首次註冊。
    once: Once,

    /// 首次註冊失敗則一直失敗。
    failed: AtomicBool,
}

impl WindowClass {
    pub(crate) const fn new() -> Self {
        Self {
            once: Once::new(),
            failed: AtomicBool::new(false),
        }
    }

    /// 首次按 `build` 註冊（`cbSize` 由這裡填）；類已存在不算失敗。
    pub(crate) fn ensure(&self, build: impl FnOnce() -> WNDCLASSEXW) -> Result<()> {
        self.once.call_once(|| {
            let mut class = build();
            class.cbSize = core::mem::size_of::<WNDCLASSEXW>() as u32;
            let atom = unsafe { RegisterClassExW(&class) };
            if atom == 0 && unsafe { GetLastError() } != ERROR_CLASS_ALREADY_EXISTS {
                self.failed.store(true, Ordering::Relaxed);
            }
        });
        if self.failed.load(Ordering::Relaxed) {
            Err(Error::from(E_FAIL))
        } else {
            Ok(())
        }
    }
}
