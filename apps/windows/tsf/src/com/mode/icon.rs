//! 工作列中 / 英 / A 圖示。SVG 預先柵格化成四檔 DPI 的 8 位 alpha 蒙版（`assets/icon/windows/render-mode-icons.sh`），
//! 這裡按系統 DPI 挑一檔、按工作列深淺色填白或填黑拼成 HICON；系統取走後負責銷燬。

use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject,
};
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO};
use windows::core::Result;

/// 三個圖示。
#[derive(Clone, Copy)]
pub(super) enum Glyph {
    Chinese,
    English,
    CapsLock,
}

/// 四檔邊長（畫素）：100% / 125% / 150% / 200% 縮放下的 16pt。
const SIZES: [usize; 4] = [16, 20, 24, 32];

macro_rules! masks {
    ($name:literal) => {
        [
            include_bytes!(concat!("../../../resources/mode/", $name, "-16.alpha")),
            include_bytes!(concat!("../../../resources/mode/", $name, "-20.alpha")),
            include_bytes!(concat!("../../../resources/mode/", $name, "-24.alpha")),
            include_bytes!(concat!("../../../resources/mode/", $name, "-32.alpha")),
        ]
    };
}

const CHINESE: [&[u8]; 4] = masks!("zh");
const ENGLISH: [&[u8]; 4] = masks!("en");
const CAPS_LOCK: [&[u8]; 4] = masks!("caps");

impl Glyph {
    fn masks(self) -> &'static [&'static [u8]; 4] {
        match self {
            Self::Chinese => &CHINESE,
            Self::English => &ENGLISH,
            Self::CapsLock => &CAPS_LOCK,
        }
    }
}

pub(super) fn make(glyph: Glyph) -> Result<HICON> {
    let index = pick_size();
    let size = SIZES[index];
    let mask = glyph.masks()[index];
    debug_assert_eq!(mask.len(), size * size);
    let ink: u32 = if taskbar_is_light() { 0x00 } else { 0xFF };
    let side = size as i32;
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: side,
            biHeight: -side, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = core::ptr::null_mut();
    unsafe {
        let color = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)?;
        // 32 位色圖按預乘 alpha 解釋。
        let pixels = std::slice::from_raw_parts_mut(bits.cast::<u32>(), size * size);
        for (pixel, &alpha) in pixels.iter_mut().zip(mask) {
            let alpha = u32::from(alpha);
            let channel = ink * alpha / 255;
            *pixel = (alpha << 24) | (channel << 16) | (channel << 8) | channel;
        }
        // 掩碼全 0，透明靠色圖的 alpha。
        let mono = CreateBitmap(side, side, 1, 1, None);
        let info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mono,
            hbmColor: color,
        };
        let icon = CreateIconIndirect(&info);
        let _ = DeleteObject(mono.into());
        let _ = DeleteObject(color.into());
        icon
    }
}

/// 16pt 換成當前 DPI 下的畫素，取不小於它的最近一檔。
fn pick_size() -> usize {
    let dpi = unsafe { GetDpiForSystem() }.max(96) as usize;
    let px = 16 * dpi / 96;
    SIZES
        .iter()
        .position(|&s| s >= px)
        .unwrap_or(SIZES.len() - 1)
}

/// 工作列淺色（`SystemUsesLightTheme = 1`）畫黑字，深色畫白字；讀不到按深色。
fn taskbar_is_light() -> bool {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_u32("SystemUsesLightTheme"))
        .is_ok_and(|value| value == 1)
}
