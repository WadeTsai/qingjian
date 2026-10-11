//! 候選視窗的點選：只有候選與譯詞那幾塊收滑鼠（`WM_NCHITTEST` 回 `HTCLIENT`），其餘地方回 `HTTRANSPARENT`
//! 穿到下面的應用，主題伸出視窗的裝飾、投影都不擋；按下不啟用視窗，點中的目標經回撥交給 Router。
//! GDI 畫法沒有點選區域，整窗穿透。

use std::cell::RefCell;

use windows::Win32::Foundation::{LPARAM, LRESULT, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, HTCLIENT, HTTRANSPARENT, MA_NOACTIVATE,
};

use qingjian_render::{HitRegion, HitTarget};

/// 點中候選或譯詞時調的回撥（UI 執行緒上調，投給工人執行緒）。
pub type CandidateClicks = Box<dyn Fn(HitTarget) + Send>;

/// 本執行緒候選視窗的點選狀態（只有一個候選視窗）。
struct Clicks {
    on_click: CandidateClicks,

    /// 最近一次貼的位圖裡的點選區域（內容區畫素）。
    hits: Vec<HitRegion>,

    /// 內容區左上角的螢幕座標（物理畫素）。
    origin: (i32, i32),
}

thread_local! {
    static CLICKS: RefCell<Option<Clicks>> = const { RefCell::new(None) };
}

/// 建視窗時登記回撥。
pub(super) fn install(on_click: CandidateClicks) {
    CLICKS.with(|clicks| {
        *clicks.borrow_mut() = Some(Clicks {
            on_click,
            hits: Vec::new(),
            origin: (0, 0),
        });
    });
}

/// 每次貼點陣圖後換上新的點選區域；GDI 畫法與收起時傳空。
pub(super) fn set_hits(hits: Vec<HitRegion>, origin: (i32, i32)) {
    CLICKS.with(|clicks| {
        if let Some(clicks) = clicks.borrow_mut().as_mut() {
            clicks.hits = hits;
            clicks.origin = origin;
        }
    });
}

/// 螢幕上這一點落在哪個目標上。
fn target_at(x: i32, y: i32) -> Option<HitTarget> {
    CLICKS.with(|clicks| {
        let clicks = clicks.borrow();
        let clicks = clicks.as_ref()?;
        let (x, y) = ((x - clicks.origin.0) as f32, (y - clicks.origin.1) as f32);
        clicks
            .hits
            .iter()
            .find(|region| region.contains(x, y))
            .map(|region| region.target)
    })
}

/// `WM_NCHITTEST`：`lparam` 是螢幕座標（有符號 16 位）。
pub(super) fn hit_test(lparam: LPARAM) -> LRESULT {
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    if target_at(x, y).is_some() {
        LRESULT(HTCLIENT as isize)
    } else {
        LRESULT(HTTRANSPARENT as isize)
    }
}

pub(super) fn mouse_activate() -> LRESULT {
    LRESULT(MA_NOACTIVATE as isize)
}

/// `WM_LBUTTONDOWN`：按當前游標的螢幕位置認目標，交給回撥。
pub(super) fn button_down() {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return;
    }
    let Some(target) = target_at(point.x, point.y) else {
        return;
    };
    tracing::debug!(?target, "點選候選視窗");
    CLICKS.with(|clicks| {
        if let Some(clicks) = clicks.borrow().as_ref() {
            (clicks.on_click)(target);
        }
    });
}
