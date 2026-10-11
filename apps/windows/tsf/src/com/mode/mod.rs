//! 中 / 英輸入模式：這裡寫 TSF 的「轉換模式」compartment，系統工作列據此顯示「中」或「英」，
//! 只翻 `TF_CONVERSIONMODE_NATIVE` 位，其它位（全 / 半形等）保留；另讀「輸入法開 / 關」compartment，
//! 系統的「輸入法/非輸入法切換」熱鍵翻的是它。反向同步在 [`sink`]；模式本身與語言欄按鈕在 [`state`] / [`button`]。

mod button;
mod icon;
pub(crate) mod menu;
pub(crate) mod sink;
mod state;

use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::TextServices::{
    GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION, GUID_COMPARTMENT_KEYBOARD_OPENCLOSE,
    ITfCompartment, ITfCompartmentMgr, ITfThreadMgr, TF_CONVERSIONMODE_NATIVE,
};
use windows::core::{Interface, Result};

use crate::com::log::log;

pub(crate) use self::button::ModeButton;
pub(crate) use self::state::ModeState;

const NATIVE: i32 = TF_CONVERSIONMODE_NATIVE as i32;

/// 失敗只記日誌：指示器不動不影響打字。
pub(crate) fn set_indicator(thread_mgr: &ITfThreadMgr, tid: u32, english: bool) {
    if let Err(error) = write_conversion_mode(thread_mgr, tid, english) {
        log(&format!("設定中英指示器失敗: {error}"));
    }
}

pub(crate) fn conversion_compartment(thread_mgr: &ITfThreadMgr) -> Result<ITfCompartment> {
    let mgr: ITfCompartmentMgr = thread_mgr.cast()?;
    unsafe { mgr.GetCompartment(&GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION) }
}

/// `NATIVE` 未點亮 = 英文。讀不到按中文起算。
pub(crate) fn is_english(compartment: &ITfCompartment) -> bool {
    read_mode(compartment) & NATIVE == 0
}

/// 「輸入法開 / 關」compartment：系統的「輸入法/非輸入法切換」熱鍵（預設 Ctrl+Space）翻的是它。
/// 只是一個狀態位，關著時按鍵照樣送來；新執行緒裡預設是關。
pub(crate) fn openclose_compartment(thread_mgr: &ITfThreadMgr) -> Result<ITfCompartment> {
    let mgr: ITfCompartmentMgr = thread_mgr.cast()?;
    unsafe { mgr.GetCompartment(&GUID_COMPARTMENT_KEYBOARD_OPENCLOSE) }
}

/// 值為 0 = 關著。沒設過 / 讀不到按開著算。
pub(crate) fn is_keyboard_open(compartment: &ITfCompartment) -> bool {
    read_i32(compartment).is_none_or(|value| value != 0)
}

/// 跟著模式寫（中文開、英文關），系統 Ctrl + Space 下一次按下才是真的切換。已是目標值就不寫，失敗只記日誌。
pub(crate) fn set_keyboard_open(thread_mgr: &ITfThreadMgr, tid: u32, open: bool) {
    let Ok(compartment) = openclose_compartment(thread_mgr) else {
        return;
    };
    if is_keyboard_open(&compartment) == open {
        return;
    }
    if let Err(error) = unsafe { compartment.SetValue(tid, &VARIANT::from(i32::from(open))) } {
        log(&format!("寫輸入法開關失敗: {error}"));
    }
}

fn write_conversion_mode(thread_mgr: &ITfThreadMgr, tid: u32, english: bool) -> Result<()> {
    let compartment = conversion_compartment(thread_mgr)?;
    let current = read_mode(&compartment);
    let next = if english {
        current & !NATIVE
    } else {
        current | NATIVE
    };
    unsafe { compartment.SetValue(tid, &VARIANT::from(next)) }
}

/// 沒設過 / 型別不對時按中文（`NATIVE` 亮）。
fn read_mode(compartment: &ITfCompartment) -> i32 {
    read_i32(compartment).unwrap_or(NATIVE)
}

fn read_i32(compartment: &ITfCompartment) -> Option<i32> {
    unsafe { compartment.GetValue() }
        .and_then(|variant| i32::try_from(&variant))
        .ok()
}
