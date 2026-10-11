//! 分層視窗合成：圓角背景 + 四周柔和陰影 + 一段 GDI 內容，合成進一張預乘 alpha 的 BGRA 點陣圖，
//! `UpdateLayeredWindow` 一次貼上。候選視窗與狀態條共用。點陣圖在 [`Canvas`]，內容圓角矩形在 [`RoundRect`]。
//! 青簡渲染器畫好的整張點陣圖（已含陰影）走 [`present`]，只做 RGBA → BGRA 再貼。

mod canvas;
mod round_rect;

use qingjian_render::Pixmap;
use windows::Win32::Foundation::{COLORREF, E_INVALIDARG, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, HDC, SetViewportOrgEx,
};
use windows::Win32::UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow};
use windows::core::{Error, Result};

use self::canvas::Canvas;
use self::round_rect::RoundRect;

/// 內容四周留給陰影的寬度（邏輯畫素）。
const SHADOW_MARGIN: i32 = 16;

/// 定向主陰影（光從上方來）的最濃 alpha：底部拿滿、兩側減半、頂部為 0。
const KEY_MAX: f64 = 65.0;

/// 環境光暈的最濃 alpha，四邊等濃。
const AMBIENT_MAX: f64 = 18.0;

/// 陰影留白的物理畫素寬度（`dpi` 96 為 100%）。
pub(super) fn shadow_margin(dpi: u32) -> i32 {
    ((SHADOW_MARGIN * dpi as i32) / 96).max(1)
}

/// 一次合成的輸入。
pub(super) struct Layered<'a> {
    /// 內容尺寸（不含陰影留白）。
    pub content: (i32, i32),

    /// 陰影留白（= [`shadow_margin`]）。
    pub margin: i32,

    /// 視窗左上角螢幕座標（= 內容左上角 − 留白）。
    pub win_pos: (i32, i32),

    /// 視窗尺寸（= 內容 + 2·留白）。
    pub win_size: (i32, i32),

    /// 內容背景色。
    pub background: COLORREF,

    /// 內容圓角半徑。
    pub corner_radius: i32,

    /// 在內容座標系裡畫內容；`client` 是 `{0, 0, w, h}`。
    pub paint: &'a dyn Fn(HDC, RECT),
}

/// 合成一幀並貼到分層視窗上。
pub(super) fn composite(hwnd: HWND, layered: &Layered) -> Result<()> {
    let (w, h) = layered.win_size;
    if w <= 0 || h <= 0 {
        return Err(Error::from(E_INVALIDARG));
    }
    let mut canvas = Canvas::new(w, h)?;
    let round = RoundRect::content(layered.content, layered.margin, layered.corner_radius);
    fill_shadow_and_background(
        canvas.pixels(),
        w,
        h,
        &round,
        layered.background,
        layered.margin,
    );

    // 視口原點挪到內容左上，內容閉包按自己的座標畫。
    let hdc = canvas.dc();
    unsafe {
        let _ = SetViewportOrgEx(hdc, layered.margin, layered.margin, None);
    }
    let client = RECT {
        left: 0,
        top: 0,
        right: layered.content.0,
        bottom: layered.content.1,
    };
    (layered.paint)(hdc, client);
    unsafe {
        let _ = SetViewportOrgEx(hdc, 0, 0, None);
    }
    // GDI 只寫 RGB、把碰到的畫素 alpha 留成 0（分層窗口裡會全透明），畫完把內容區補回 255。
    restore_content_alpha(canvas.pixels(), w, h, &round);
    update(hwnd, canvas.dc(), layered.win_pos, (w, h))
}

/// 把渲染器出的預乘 RGBA 點陣圖（已含陰影邊）貼到分層視窗上，`win_pos` 是點陣圖左上角的螢幕座標。
pub(super) fn present(hwnd: HWND, pixmap: &Pixmap, win_pos: (i32, i32)) -> Result<()> {
    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    if w <= 0 || h <= 0 {
        return Err(Error::from(E_INVALIDARG));
    }
    let mut canvas = Canvas::new(w, h)?;
    // tiny-skia 是 RGBA，DIB 是 BGRA；都是預乘，只換通道順序。
    for (dst, src) in canvas.pixels().chunks_exact_mut(4).zip(pixmap.pixels()) {
        dst[0] = src.blue();
        dst[1] = src.green();
        dst[2] = src.red();
        dst[3] = src.alpha();
    }
    update(hwnd, canvas.dc(), win_pos, (w, h))
}

/// `UpdateLayeredWindow`：整張點陣圖按預乘 alpha 貼上並挪到 `win_pos`。
fn update(hwnd: HWND, hdc: HDC, win_pos: (i32, i32), win_size: (i32, i32)) -> Result<()> {
    let (w, h) = win_size;
    let dst = POINT {
        x: win_pos.0,
        y: win_pos.1,
    };
    let size = SIZE { cx: w, cy: h };
    let src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    unsafe {
        UpdateLayeredWindow(
            hwnd,
            None,
            Some(&dst),
            Some(&size),
            Some(hdc),
            Some(&src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        )
    }
}

/// 圓角矩形內填背景色（alpha 255），外面畫黑色陰影。陰影純黑、背景不透明，所以不用真做預乘。
/// 陰影從內容邊緣就開始淡出，不偏移、不填實心色帶（否則底邊多一道生硬暗帶）。
fn fill_shadow_and_background(
    pixels: &mut [u8],
    w: i32,
    h: i32,
    round: &RoundRect,
    background: COLORREF,
    margin: i32,
) {
    // COLORREF 低位到高位是 R、G、B；DIB 每畫素是 B、G、R、A。
    let bg = background.0;
    let bg_r = (bg & 0xFF) as u8;
    let bg_g = ((bg >> 8) & 0xFF) as u8;
    let bg_b = ((bg >> 16) & 0xFF) as u8;
    let margin = (margin as f64).max(1.0);
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let d = round.distance(x, y);
            if d <= 0.0 {
                pixels[idx] = bg_b;
                pixels[idx + 1] = bg_g;
                pixels[idx + 2] = bg_r;
                pixels[idx + 3] = 255;
            } else {
                let fall = (1.0 - d / margin).max(0.0);
                let fall = fall * fall;
                let ambient = AMBIENT_MAX * fall;
                let key = KEY_MAX * round.down_weight(x, y) * fall;
                pixels[idx] = 0;
                pixels[idx + 1] = 0;
                pixels[idx + 2] = 0;
                pixels[idx + 3] = (ambient + key).min(255.0) as u8;
            }
        }
    }
}

/// 把內容區所有畫素 alpha 補成 255。
fn restore_content_alpha(pixels: &mut [u8], w: i32, h: i32, round: &RoundRect) {
    for y in 0..h {
        for x in 0..w {
            if round.distance(x, y) <= 0.0 {
                let idx = ((y * w + x) * 4) as usize;
                pixels[idx + 3] = 255;
            }
        }
    }
}
