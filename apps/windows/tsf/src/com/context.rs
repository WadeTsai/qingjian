//! 文件上下文級的開關：TSF 規定鍵盤類文本服務**必須**看 `GUID_COMPARTMENT_KEYBOARD_DISABLED`（非零 = 所有鍵原樣放行、不組句），
//! 微軟 SampleIME 連同 `GUID_COMPARTMENT_EMPTYCONTEXT` 一起查。密碼框走的就是這條：微軟文件明說密碼框應當停用文本服務
//! （`IS_PASSWORD` 只是標註、不提供保護），Chromium 系瀏覽器給密碼框的上下文設的也是它。對應 macOS 的 Secure Input：直接不組句。

use windows::Win32::UI::TextServices::{
    GUID_COMPARTMENT_EMPTYCONTEXT, GUID_COMPARTMENT_KEYBOARD_DISABLED, ITfCompartmentMgr,
    ITfContext,
};
use windows::core::{GUID, Interface};

/// 這個上下文禁了鍵盤輸入（密碼框、空上下文）：所有鍵放行。讀不到按沒禁。
pub(crate) fn keyboard_disabled(context: &ITfContext) -> bool {
    let Ok(manager) = context.cast::<ITfCompartmentMgr>() else {
        return false;
    };
    flag(&manager, &GUID_COMPARTMENT_KEYBOARD_DISABLED)
        || flag(&manager, &GUID_COMPARTMENT_EMPTYCONTEXT)
}

/// 上下文 compartment 裡的 `DWORD` 非零。沒設過 / 型別不對按 0。
fn flag(manager: &ITfCompartmentMgr, guid: &GUID) -> bool {
    unsafe { manager.GetCompartment(guid) }
        .and_then(|compartment| unsafe { compartment.GetValue() })
        .ok()
        .and_then(|variant| i32::try_from(&variant).ok())
        .is_some_and(|value| value != 0)
}
