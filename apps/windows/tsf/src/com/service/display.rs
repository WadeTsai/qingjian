//! `ITfDisplayAttributeProvider`：把組句下劃線的顯示屬性（[`display_attribute`]）交給系統。

use windows::Win32::Foundation::E_INVALIDARG;
use windows::Win32::UI::TextServices::{
    IEnumTfDisplayAttributeInfo, ITfDisplayAttributeInfo, ITfDisplayAttributeProvider_Impl,
};
use windows::core::{GUID, Result};

use super::TextService_Impl;
use crate::com::display_attribute;

/// 系統按 `GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER` 類別來查組句樣式（內聯下劃線）。
impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(display_attribute::enumerator())
    }

    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> Result<ITfDisplayAttributeInfo> {
        if unsafe { *guid } == display_attribute::GUID_DISPLAY_ATTRIBUTE_INPUT {
            Ok(display_attribute::info())
        } else {
            Err(E_INVALIDARG.into())
        }
    }
}
