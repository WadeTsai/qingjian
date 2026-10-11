//! 反向同步：監聽「轉換模式」（使用者點工作列中 / 英）與「輸入法開 / 關」（系統 Ctrl + Space）兩條 compartment，
//! 變了讓 service 跟著改中英模式。回撥裡只讀不寫；自己寫觸發的那次讀回與當前相同，service 裡忽略。

use windows::Win32::UI::TextServices::{
    GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION, GUID_COMPARTMENT_KEYBOARD_OPENCLOSE,
    ITfCompartment, ITfCompartmentEventSink, ITfCompartmentEventSink_Impl, ITfSource, ITfThreadMgr,
};
use windows::core::{GUID, Interface, Result, implement};

use crate::com::log::log;
use crate::com::service;

#[implement(ITfCompartmentEventSink)]
struct SystemModeSink;

impl ITfCompartmentEventSink_Impl for SystemModeSink_Impl {
    fn OnChange(&self, rguid: *const GUID) -> Result<()> {
        let guid = unsafe { *rguid };
        if guid == GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION {
            service::on_conversion_mode_changed();
        } else if guid == GUID_COMPARTMENT_KEYBOARD_OPENCLOSE {
            service::on_keyboard_open_changed();
        }
        Ok(())
    }
}

/// 掛上的回撥（source + cookie），停用時 [`unadvise`]。
pub(crate) type Advice = Vec<(ITfSource, u32)>;

/// 兩條 compartment 各掛一個回撥；一條掛不上只記日誌，另一條照常。
pub(crate) fn advise(thread_mgr: &ITfThreadMgr) -> Advice {
    let sink: ITfCompartmentEventSink = SystemModeSink.into();
    let targets = [
        ("轉換模式", super::conversion_compartment(thread_mgr)),
        ("輸入法開 / 關", super::openclose_compartment(thread_mgr)),
    ];
    targets
        .into_iter()
        .filter_map(|(what, compartment)| {
            compartment
                .and_then(|compartment| advise_one(&compartment, &sink))
                .map_err(|error| log(&format!("監聽{what}失敗: {error}")))
                .ok()
        })
        .collect()
}

fn advise_one(
    compartment: &ITfCompartment,
    sink: &ITfCompartmentEventSink,
) -> Result<(ITfSource, u32)> {
    let source: ITfSource = compartment.cast()?;
    let cookie = unsafe { source.AdviseSink(&ITfCompartmentEventSink::IID, sink)? };
    Ok((source, cookie))
}

pub(crate) fn unadvise(advice: Advice) {
    for (source, cookie) in advice {
        let _ = unsafe { source.UnadviseSink(cookie) };
    }
}
