//! 工作列「中 / 英」圖示的右鍵選單。做法同小狼毫：`TrackPopupMenuEx` 掛在輸入框所在視窗上，同步取回點的項。

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, MENU_ITEM_FLAGS, MF_CHECKED, MF_GRAYED,
    MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenuEx,
};
use windows::core::{HSTRING, PCWSTR};

use qingjian_platform::protocol::IndicatorCommand;

/// 開啟選單時的勾選狀態。
pub(crate) struct MenuState {
    pub(crate) english: bool,
    pub(crate) english_enabled: bool,
    pub(crate) full_width: bool,
    pub(crate) status_bar: bool,
    pub(crate) update_available: bool,
}

/// 點了哪一項：中 / 英 DLL 自己切，其餘交給 Server。
pub(crate) enum MenuChoice {
    Mode { english: bool },
    Server(IndicatorCommand),
}

const ID_CHINESE: u32 = 1;
const ID_ENGLISH: u32 = 2;
const ID_PUNCTUATION: u32 = 3;
const ID_STATUS_BAR: u32 = 4;
const ID_SETTINGS: u32 = 5;
const ID_DOWNLOAD: u32 = 6;

/// 在 `point`（螢幕座標）彈出選單，阻塞到使用者點了某項或點別處關掉。
pub(crate) fn track(owner: HWND, point: POINT, state: &MenuState) -> Option<MenuChoice> {
    let checked = |on: bool| if on { MF_CHECKED } else { MENU_ITEM_FLAGS(0) };
    let english_flags = if state.english_enabled {
        checked(state.english)
    } else {
        MF_GRAYED
    };
    let mut items = vec![
        Some((ID_CHINESE, "中文", checked(!state.english))),
        Some((ID_ENGLISH, "英文", english_flags)),
        None,
        Some((ID_PUNCTUATION, "全形標點", checked(state.full_width))),
        Some((ID_STATUS_BAR, "懸浮狀態條", checked(state.status_bar))),
        None,
    ];
    if state.update_available {
        items.push(Some((
            ID_DOWNLOAD,
            "有新版本，前往下載…",
            MENU_ITEM_FLAGS(0),
        )));
    }
    items.push(Some((ID_SETTINGS, "設定…", MENU_ITEM_FLAGS(0))));
    let menu = unsafe { CreatePopupMenu() }.ok()?;
    for item in items {
        let _ = match item {
            Some((id, text, flags)) => unsafe {
                AppendMenuW(menu, MF_STRING | flags, id as usize, &HSTRING::from(text))
            },
            None => unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) },
        };
    }
    // 圖示在工作列上，選單往上彈
    let flags = TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN;
    let id = unsafe { TrackPopupMenuEx(menu, flags.0, point.x, point.y, owner, None) };
    let _ = unsafe { DestroyMenu(menu) };
    match id.0 as u32 {
        ID_CHINESE => Some(MenuChoice::Mode { english: false }),
        ID_ENGLISH => Some(MenuChoice::Mode { english: true }),
        ID_PUNCTUATION => Some(MenuChoice::Server(IndicatorCommand::TogglePunctuation)),
        ID_STATUS_BAR => Some(MenuChoice::Server(IndicatorCommand::ToggleStatusBar)),
        ID_SETTINGS => Some(MenuChoice::Server(IndicatorCommand::OpenSettings)),
        ID_DOWNLOAD => Some(MenuChoice::Server(IndicatorCommand::OpenDownload)),
        _ => None,
    }
}
