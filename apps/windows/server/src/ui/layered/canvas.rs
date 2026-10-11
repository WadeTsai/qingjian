//! 合成用的點陣圖：一張 top-down 32bpp DIB 與記憶體 DC。

use windows::Win32::Foundation::E_FAIL;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GdiFlush, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows::core::{Error, Result};

/// 一張 top-down 32bpp DIB 與選進了它的記憶體 DC。畫素只能經 [`Self::pixels`]（`&mut self`）訪問，
/// 保證 GDI 在畫的時候沒有 Rust 側切片指著同一塊記憶體。
pub(super) struct Canvas {
    dib: HBITMAP,
    memdc: HDC,
    previous: HGDIOBJ,
    bits: *mut u8,
    len: usize,
}

impl Canvas {
    pub(super) fn new(w: i32, h: i32) -> Result<Self> {
        let header = BITMAPINFOHEADER {
            biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // 負高 = top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let bmi = BITMAPINFO {
            bmiHeader: header,
            ..Default::default()
        };
        let mut bits = core::ptr::null_mut();
        let dib = unsafe { CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)? };
        let (memdc, previous) = unsafe {
            let memdc = CreateCompatibleDC(None);
            (memdc, SelectObject(memdc, dib.into()))
        };
        if bits.is_null() || memdc.is_invalid() {
            unsafe {
                let _ = DeleteDC(memdc);
                let _ = DeleteObject(dib.into());
            }
            return Err(Error::from(E_FAIL));
        }
        Ok(Self {
            dib,
            memdc,
            previous,
            bits: bits.cast(),
            len: (w * h * 4) as usize,
        })
    }

    pub(super) fn dc(&self) -> HDC {
        self.memdc
    }

    /// 畫素（BGRA，top-down）。先 `GdiFlush` 把批次的 GDI 呼叫落到點陣圖上。
    pub(super) fn pixels(&mut self) -> &mut [u8] {
        // SAFETY: 緩衝由 CreateDIBSection 分配、w*h*4 位元組連續、隨 dib 存活；`&mut self` 保證獨佔。
        unsafe {
            let _ = GdiFlush();
            std::slice::from_raw_parts_mut(self.bits, self.len)
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.memdc, self.previous);
            let _ = DeleteDC(self.memdc);
            let _ = DeleteObject(self.dib.into());
        }
    }
}
