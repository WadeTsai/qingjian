//! 組句 preedit 的內聯下劃線：按 TSF 顯示屬性協議給組句範圍標一個「輸入中」屬性（對應 macOS marked text 的下劃線）。
//! 系統經 `ITfDisplayAttributeProvider`（實現在 [`super::service::TextService`]）來取 [`AttributeInfo`]
//! （列舉器 [`AttributeEnum`]）；寫組句時把 GUID 換成 atom 寫進範圍的 `GUID_PROP_ATTRIBUTE`。

mod enumerator;
mod info;

use core::cell::Cell;

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, GUID_PROP_ATTRIBUTE, IEnumTfDisplayAttributeInfo, ITfCategoryMgr,
    ITfContext, ITfDisplayAttributeInfo, ITfRange, TF_ATTR_INPUT, TF_CT_NONE, TF_DA_COLOR,
    TF_DISPLAYATTRIBUTE, TF_LS_DOT,
};
use windows::core::{GUID, Result};

use self::enumerator::AttributeEnum;
use self::info::AttributeInfo;
use super::log::log;

/// 青簡的組句顯示屬性 GUID（自定義），與登錄檔裡宣告的顯示屬性提供者類別配套。
pub(crate) const GUID_DISPLAY_ATTRIBUTE_INPUT: GUID =
    GUID::from_u128(0xc47cb4c0_0ac9_4c8f_bdbf_8b6d21cc504f);

thread_local! {
    /// GUID 在類別管理器裡的 atom，首次用時算出來快取；0 = 還沒算。
    static ATOM: Cell<u32> = const { Cell::new(0) };
}

/// 點虛線下劃線，前景 / 背景 / 線色跟隨文件。
pub(super) fn input_attribute() -> TF_DISPLAYATTRIBUTE {
    let follow_text = TF_DA_COLOR {
        r#type: TF_CT_NONE,
        ..Default::default()
    };
    TF_DISPLAYATTRIBUTE {
        crText: follow_text,
        crBk: follow_text,
        lsStyle: TF_LS_DOT,
        fBoldLine: false.into(),
        crLine: follow_text,
        bAttr: TF_ATTR_INPUT,
    }
}

/// 給組句範圍打上下劃線屬性。失敗只記日誌。
pub(crate) fn mark(context: &ITfContext, ec: u32, range: &ITfRange) {
    if let Err(error) = mark_inner(context, ec, range) {
        log(&format!("組句下劃線屬性寫入失敗: {error}"));
    }
}

fn mark_inner(context: &ITfContext, ec: u32, range: &ITfRange) -> Result<()> {
    let variant = VARIANT::from(guid_atom()? as i32);
    let property = unsafe { context.GetProperty(&GUID_PROP_ATTRIBUTE)? };
    unsafe { property.SetValue(ec, range, &variant) }
}

fn guid_atom() -> Result<u32> {
    let cached = ATOM.with(Cell::get);
    if cached != 0 {
        return Ok(cached);
    }
    let manager: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
    let atom = unsafe { manager.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE_INPUT)? };
    ATOM.with(|a| a.set(atom));
    Ok(atom)
}

pub(crate) fn enumerator() -> IEnumTfDisplayAttributeInfo {
    AttributeEnum {
        done: Cell::new(false),
    }
    .into()
}

pub(crate) fn info() -> ITfDisplayAttributeInfo {
    AttributeInfo.into()
}
