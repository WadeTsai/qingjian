//! 狀態條的擺放狀態與點選分發：視窗過程按 HWND 查到它，拖動結束記位置、點選按格分發動作（[`StatusAction`]）。

mod action;

use std::cell::{Cell, RefCell};

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

pub(super) use self::action::StatusAction;
use crate::dispatch::StatusEvent;
use crate::ui::StatusEvents;

/// 視窗過程也要讀寫的擺放狀態，經 `PLACEMENTS` 按 HWND 查到。
pub(super) struct Placement {
    /// 狀態條視窗。
    pub(super) hwnd: HWND,

    /// 上次合成用的陰影留白，拖動結束時從視窗矩形反推內容左上角用；點選時把客戶區座標換成內容座標。
    pub(super) margin: Cell<i32>,

    /// 內容左上角的螢幕座標（物理畫素）；`None` 表示還沒擺放過。
    pub(super) pos: Cell<Option<(i32, i32)>>,

    /// 上次畫出的各格右邊界（內容座標）與動作，從左到右；點選按 x 落進哪格。
    pub(super) cells: RefCell<Vec<(i32, StatusAction)>>,

    /// 前臺全屏、暫時收起了（見 [`super::fullscreen`]）。
    pub(super) fullscreen_hidden: Cell<bool>,

    /// 點格 / 拖動結束回給 Router。
    events: StatusEvents,
}

impl Placement {
    pub(super) fn new(hwnd: HWND, margin: i32, events: StatusEvents) -> Self {
        Self {
            hwnd,
            margin: Cell::new(margin),
            pos: Cell::new(None),
            cells: RefCell::new(Vec::new()),
            fullscreen_hidden: Cell::new(false),
            events,
        }
    }

    /// 拖動結束：從視窗矩形反推內容左上角，記下並交給 Router 寫回配置。
    pub(super) fn on_moved(&self) {
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut rect) }.is_ok() {
            let margin = self.margin.get();
            let x = rect.left + margin;
            let y = rect.top + margin;
            self.pos.set(Some((x, y)));
            (self.events)(StatusEvent::Moved(x, y));
        }
    }

    /// 單擊：`client_x` 是客戶區橫座標。
    pub(super) fn on_click(&self, client_x: i32) {
        let x = client_x - self.margin.get();
        let action = self
            .cells
            .borrow()
            .iter()
            .find(|(right, _)| x < *right)
            .map(|(_, action)| *action);
        match action {
            Some(StatusAction::ToggleMode) => (self.events)(StatusEvent::ToggleMode),
            Some(StatusAction::TogglePunctuation) => {
                (self.events)(StatusEvent::TogglePunctuation);
            }
            Some(StatusAction::OpenSettings) => crate::ui::open_settings(),
            None => {}
        }
    }
}
