//! 青簡唯一的顯示屬性描述（組句「輸入中」的點虛線下劃線）；使用者不可改樣式。

use windows::Win32::UI::TextServices::{
    ITfDisplayAttributeInfo, ITfDisplayAttributeInfo_Impl, TF_DISPLAYATTRIBUTE,
};
use windows::core::{BSTR, GUID, Result, implement};

use super::{GUID_DISPLAY_ATTRIBUTE_INPUT, input_attribute};

/// 青簡唯一的顯示屬性描述；使用者不可改樣式。
#[implement(ITfDisplayAttributeInfo)]
pub(super) struct AttributeInfo;

impl ITfDisplayAttributeInfo_Impl for AttributeInfo_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(GUID_DISPLAY_ATTRIBUTE_INPUT)
    }

    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("青簡拼音"))
    }

    fn GetAttributeInfo(&self, pda: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        unsafe { *pda = input_attribute() };
        Ok(())
    }

    fn SetAttributeInfo(&self, _pda: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        Ok(())
    }
}
