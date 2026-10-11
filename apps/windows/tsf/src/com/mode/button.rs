//! 中 / 英輸入模式指示器：Win11 托盤品牌圖示左邊的模式圖示。按微軟 IME 的做法經 `GUID_LBI_INPUTMODE`
//! 語言欄按鈕把圖示交給系統（轉換模式 compartment 不走這條通道，光寫它不顯示）。Caps Lock 亮著顯示「A」。

use std::rc::Rc;

use windows::Win32::Foundation::{E_NOINTERFACE, POINT, RECT};
use windows::Win32::UI::TextServices::{
    GUID_LBI_INPUTMODE, ITfLangBarItem_Impl, ITfLangBarItemButton, ITfLangBarItemButton_Impl,
    ITfLangBarItemSink, ITfMenu, ITfSource, ITfSource_Impl, TF_LANGBARITEMINFO, TF_LBI_CLK_RIGHT,
    TF_LBI_STYLE_BTN_BUTTON, TfLBIClick,
};
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::core::{BOOL, BSTR, GUID, IUnknown, Interface, Ref, Result, implement};

use qingjian_platform::SwitchKeys;

use super::ModeState;
use super::icon::{self, Glyph};
use crate::com::CLSID_QINGJIAN;
use crate::com::key::event::caps_lock_on;

/// `GUID_LBI_INPUTMODE` 語言欄按鈕：圖示隨 [`ModeState`] 顯示中 / 英，點它切模式。
#[implement(ITfLangBarItemButton, ITfSource)]
pub(crate) struct ModeButton {
    state: Rc<ModeState>,
}

impl ModeButton {
    pub(crate) fn create(state: Rc<ModeState>) -> ITfLangBarItemButton {
        Self { state }.into()
    }
}

impl ITfLangBarItem_Impl for ModeButton_Impl {
    fn GetInfo(&self, pinfo: *mut TF_LANGBARITEMINFO) -> Result<()> {
        let info = unsafe { &mut *pinfo };
        info.clsidService = CLSID_QINGJIAN;
        info.guidItem = GUID_LBI_INPUTMODE;
        info.dwStyle = TF_LBI_STYLE_BTN_BUTTON;
        info.ulSort = 0;
        let desc: Vec<u16> = "青簡中英模式".encode_utf16().collect();
        let n = desc.len().min(info.szDescription.len());
        info.szDescription[..n].copy_from_slice(&desc[..n]);
        Ok(())
    }

    fn GetStatus(&self) -> Result<u32> {
        Ok(0)
    }

    fn Show(&self, _fshow: BOOL) -> Result<()> {
        Ok(())
    }

    fn GetTooltipString(&self) -> Result<BSTR> {
        let keys = self.state.switch_keys();
        let text = if !self.state.enabled() {
            "中 / 英（內建英文模式已關閉）".to_owned()
        } else if keys == SwitchKeys::NONE {
            "中 / 英（未設切換鍵，點這裡切換）".to_owned()
        } else {
            format!("中 / 英（{}切換）", keys.describe())
        };
        Ok(BSTR::from(text))
    }
}

impl ITfLangBarItemButton_Impl for ModeButton_Impl {
    /// 左鍵切中英，右鍵彈選單（中 / 英、全形標點、懸浮狀態條、設定）。
    fn OnClick(&self, click: TfLBIClick, pt: &POINT, _prcarea: *const RECT) -> Result<()> {
        if click == TF_LBI_CLK_RIGHT {
            crate::com::service::show_indicator_menu(*pt);
        } else {
            crate::com::service::toggle_mode();
        }
        Ok(())
    }

    fn InitMenu(&self, _pmenu: Ref<ITfMenu>) -> Result<()> {
        Ok(())
    }

    fn OnMenuSelect(&self, _wid: u32) -> Result<()> {
        Ok(())
    }

    fn GetIcon(&self) -> Result<HICON> {
        icon::make(self.glyph())
    }

    fn GetText(&self) -> Result<BSTR> {
        Ok(BSTR::from(match self.glyph() {
            Glyph::Chinese => "中",
            Glyph::English => "英",
            Glyph::CapsLock => "A",
        }))
    }
}

impl ModeButton_Impl {
    /// Caps 亮著無論中英模式都直接出大寫英文，所以它優先。
    fn glyph(&self) -> Glyph {
        if caps_lock_on() {
            Glyph::CapsLock
        } else if self.state.english() {
            Glyph::English
        } else {
            Glyph::Chinese
        }
    }
}

impl ITfSource_Impl for ModeButton_Impl {
    fn AdviseSink(&self, riid: *const GUID, punk: Ref<IUnknown>) -> Result<u32> {
        if unsafe { *riid } != ITfLangBarItemSink::IID {
            return Err(E_NOINTERFACE.into());
        }
        let sink: ITfLangBarItemSink = punk.ok()?.cast()?;
        *self.state.sink.borrow_mut() = Some(sink);
        Ok(1) // 只支援一個回撥，cookie 固定
    }

    fn UnadviseSink(&self, _dwcookie: u32) -> Result<()> {
        *self.state.sink.borrow_mut() = None;
        Ok(())
    }
}
