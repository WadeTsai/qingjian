//! 按字族名找系統裡的字型檔案（CoreText 的字型登記），給渲染器只加載這幾個檔案；再列出可選的字族名給設定頁。

use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::NSFontManager;
use objc2_core_foundation::{CFArray, CFDictionary, CFRetained, CFString, CFURL, CFURLPathStyle};
use objc2_core_text::{CTFontDescriptor, kCTFontFamilyNameAttribute, kCTFontURLAttribute};

/// 某字族所有面的檔案路徑，去重；系統裡沒有這個字族返回空。
pub(crate) fn family_files(family: &str) -> Vec<PathBuf> {
    // SAFETY: 只讀 CoreText 匯出的屬性名常量；屬性字典鍵值都是 CFString，與 CoreText 的約定一致
    let descriptors = unsafe {
        let key: &CFString = kCTFontFamilyNameAttribute;
        let value = CFString::from_str(family);
        let attributes = CFDictionary::from_slices(&[key], &[&*value]);
        let descriptor = CTFontDescriptor::with_attributes(attributes.as_opaque());
        descriptor.matching_font_descriptors(None)
    };
    let mut files: Vec<PathBuf> = Vec::new();
    let Some(descriptors) = descriptors else {
        return files;
    };
    // SAFETY: CTFontDescriptorCreateMatchingFontDescriptors 返回的陣列元素就是 CTFontDescriptor
    let descriptors: CFRetained<CFArray<CTFontDescriptor>> =
        unsafe { CFRetained::cast_unchecked(descriptors) };
    for descriptor in descriptors.iter() {
        // SAFETY: 只讀匯出的常量名
        let url = unsafe { descriptor.attribute(kCTFontURLAttribute) }
            .and_then(|value| value.downcast::<CFURL>().ok());
        let Some(path) =
            url.and_then(|url| url.file_system_path(CFURLPathStyle::CFURLPOSIXPathStyle))
        else {
            continue;
        };
        let path = PathBuf::from(path.to_string());
        if !files.contains(&path) {
            files.push(path);
        }
    }
    files
}

/// 系統裡可選的字族名，按系統給的順序。
pub(crate) fn available_families(mtm: MainThreadMarker) -> Vec<String> {
    NSFontManager::sharedFontManager(mtm)
        .availableFontFamilies()
        .iter()
        .map(|name| name.to_string())
        // 以點開頭的是系統私有字型，使用者選不著也不該列
        .filter(|name| !name.starts_with('.'))
        .collect()
}
