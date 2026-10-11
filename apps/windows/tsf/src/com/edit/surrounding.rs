//! 組句起始時讀應用游標前的文字，給本地整句模型當前文（對應 macOS 殼的 `surrounding_text`），
//! 順手按輸入範圍判這個輸入框私密不私密（[`private_input`]）。在起組句的那次讀寫會話裡做（此時選區還是原來的插入點，
//! 拼音還沒插進去），不另開會話。

use std::mem::ManuallyDrop;

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Variant::VT_UNKNOWN;
use windows::Win32::UI::TextServices::{
    GUID_PROP_INPUTSCOPE, IS_ALPHANUMERIC_PIN, IS_NUMERIC_PASSWORD, IS_NUMERIC_PIN, IS_PASSWORD,
    IS_PRIVATE, ITfContext, ITfInputScope, ITfRange, InputScope, TF_ANCHOR_START,
    TF_DEFAULT_SELECTION, TF_SELECTION,
};
use windows::core::Interface;

/// 往前讀多少字（與 macOS 殼的 `RESCORE_LOOKBACK` 一致）。
const LOOKBACK: i32 = 64;

/// 起組句時對輸入框的判斷：私密不私密，以及不私密時游標前的文字。
pub(crate) struct InputContext {
    /// 輸入範圍聲明瞭私密 / 密碼 / PIN（[`SECRET_SCOPES`]）：不讀前文，Server 側不學不記不發雲端。
    pub(crate) private: bool,

    /// 當前選區起點之前最多 [`LOOKBACK`] 個 UTF-16 單元的文本。私密、沒有選區、讀不到時為 `None`。
    pub(crate) before: Option<String>,
}

/// 起組句時讀一次：先判私密，不私密再讀前文。
pub(crate) fn input_context(context: &ITfContext, ec: u32) -> InputContext {
    let Some(range) = selection_start(context, ec) else {
        return InputContext {
            private: false,
            before: None,
        };
    };
    if private_input(context, ec, &range) {
        crate::com::log::log("私密輸入框，不讀游標前文");
        return InputContext {
            private: true,
            before: None,
        };
    }
    InputContext {
        private: false,
        before: text_before_caret(context, ec, range),
    }
}

/// `range`（已折成插入點）之前最多 [`LOOKBACK`] 個 UTF-16 單元的文本；讀不到 / 為空是 `None`。
fn text_before_caret(context: &ITfContext, ec: u32, range: ITfRange) -> Option<String> {
    let _ = context;
    let mut shifted = 0i32;
    unsafe { range.ShiftStart(ec, -LOOKBACK, &mut shifted, std::ptr::null()) }.ok()?;
    if shifted == 0 {
        return None;
    }
    let mut buf = [0u16; LOOKBACK as usize];
    let mut fetched = 0u32;
    unsafe { range.GetText(ec, 0, &mut buf, &mut fetched) }.ok()?;
    let text = String::from_utf16_lossy(&buf[..fetched as usize]);
    (!text.is_empty()).then_some(text)
}

/// 選區折成起點（插入點）。
fn selection_start(context: &ITfContext, ec: u32) -> Option<ITfRange> {
    let mut selection = [TF_SELECTION::default()];
    let mut fetched = 0u32;
    unsafe {
        context
            .GetSelection(ec, TF_DEFAULT_SELECTION, &mut selection, &mut fetched)
            .ok()?;
    }
    if fetched == 0 {
        return None;
    }
    // GetSelection 移交 range 的所有權（ManuallyDrop），取出後由這裡釋放。
    let range = unsafe { ManuallyDrop::take(&mut selection[0].range) }?;
    unsafe { range.Collapse(ec, TF_ANCHOR_START) }.ok()?;
    Some(range)
}

/// 算作私密的輸入範圍：密碼 / PIN 之外還有 `IS_PRIVATE`——Chromium（Edge / Chrome）給密碼框與無痕窗口裡所有輸入框報的
/// 都是它（含義是「別學」），不是 `IS_PASSWORD`。真正的密碼框另有 `KEYBOARD_DISABLED` compartment 讓整鍵放行
/// （見 [`crate::com::context`]），到不了這裡；這裡兜的是沒禁鍵盤但聲明瞭私密的輸入框：照常組句，但不讀前文、不學、不記、不發雲端。
const SECRET_SCOPES: [InputScope; 5] = [
    IS_PASSWORD,
    IS_PRIVATE,
    IS_NUMERIC_PASSWORD,
    IS_NUMERIC_PIN,
    IS_ALPHANUMERIC_PIN,
];

/// 輸入框聲明瞭私密類輸入範圍（`GUID_PROP_INPUTSCOPE` 裡含 [`SECRET_SCOPES`] 之一）。拿不到屬性按不私密。
fn private_input(context: &ITfContext, ec: u32, range: &ITfRange) -> bool {
    match input_scopes(context, ec, range) {
        Ok(scopes) => {
            crate::com::log::log(&format!("輸入範圍: {scopes:?}"));
            scopes.iter().any(|scope| SECRET_SCOPES.contains(scope))
        }
        // 不支援輸入範圍屬性的應用（如記事本）GetValue 會失敗，按不私密，不記日誌
        Err(_) => false,
    }
}

/// 應用給 `range` 宣告的全部輸入範圍；哪一步拿不到就說哪一步。
fn input_scopes(
    context: &ITfContext,
    ec: u32,
    range: &ITfRange,
) -> Result<Vec<InputScope>, String> {
    let property = unsafe { context.GetAppProperty(&GUID_PROP_INPUTSCOPE) }
        .map_err(|error| format!("GetAppProperty {error}"))?;
    let value =
        unsafe { property.GetValue(ec, range) }.map_err(|error| format!("GetValue {error}"))?;
    // SAFETY: 只在 vt 是 VT_UNKNOWN 時讀 punkVal 那個聯合體成員。
    let scope: ITfInputScope = unsafe {
        let inner = &value.Anonymous.Anonymous;
        if inner.vt != VT_UNKNOWN {
            return Err(format!("vt={}", inner.vt.0));
        }
        inner
            .Anonymous
            .punkVal
            .as_ref()
            .ok_or_else(|| "punkVal 空".to_owned())?
            .cast()
            .map_err(|error| format!("cast ITfInputScope {error}"))?
    };
    let mut scopes: *mut InputScope = std::ptr::null_mut();
    let mut count = 0u32;
    unsafe { scope.GetInputScopes(&mut scopes, &mut count) }
        .map_err(|error| format!("GetInputScopes {error}"))?;
    if scopes.is_null() {
        return Err("GetInputScopes 返回空陣列".to_owned());
    }
    // SAFETY: GetInputScopes 返回 count 個元素的 CoTaskMem 陣列，由呼叫方釋放。
    unsafe {
        let list = std::slice::from_raw_parts(scopes, count as usize).to_vec();
        CoTaskMemFree(Some(scopes.cast()));
        Ok(list)
    }
}
