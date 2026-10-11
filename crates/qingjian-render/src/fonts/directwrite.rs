//! Windows 的字型登記（DirectWrite）：按字族名找出字型檔案給渲染器只加載這幾個，再列出字族名給設定頁。
//! 與 macOS 殼裡走 CoreText 的 `font_files.rs` 對應。

use std::path::PathBuf;

use ::windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection,
    IDWriteFontFile, IDWriteLocalFontFileLoader, IDWriteLocalizedStrings,
};
use ::windows::core::{BOOL, HSTRING, Interface};

/// 系統字型集合；DirectWrite 不可用時為 `None`。
fn system_collection() -> Option<IDWriteFontCollection> {
    // SAFETY: 共享工廠隨程序存活；出參按文件傳指標。
    unsafe {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).ok()?;
        let mut collection = None;
        factory
            .GetSystemFontCollection(&mut collection, false)
            .ok()?;
        collection
    }
}

/// 某字族所有面的檔案路徑，去重；系統裡沒有這個字族返回空。
pub fn family_files(family: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    let Some(collection) = system_collection() else {
        return files;
    };
    // SAFETY: 都是 DirectWrite 的只讀查詢；出參緩衝按它報的長度分配。
    unsafe {
        let mut index = 0u32;
        let mut exists = BOOL(0);
        if collection
            .FindFamilyName(&HSTRING::from(family), &mut index, &mut exists)
            .is_err()
            || !exists.as_bool()
        {
            return files;
        }
        let Ok(family) = collection.GetFontFamily(index) else {
            return files;
        };
        for i in 0..family.GetFontCount() {
            let Ok(face) = family.GetFont(i).and_then(|font| font.CreateFontFace()) else {
                continue;
            };
            let mut count = 0u32;
            if face.GetFiles(&mut count, None).is_err() || count == 0 {
                continue;
            }
            let mut handles: Vec<Option<IDWriteFontFile>> = vec![None; count as usize];
            if face
                .GetFiles(&mut count, Some(handles.as_mut_ptr()))
                .is_err()
            {
                continue;
            }
            for file in handles.into_iter().flatten() {
                if let Some(path) = file_path(&file)
                    && !files.contains(&path)
                {
                    files.push(path);
                }
            }
        }
    }
    files
}

/// 本機字型檔案的路徑；網路 / 記憶體字型（不是本地載入器）返回 `None`。
fn file_path(file: &IDWriteFontFile) -> Option<PathBuf> {
    // SAFETY: 引用鍵由 DirectWrite 持有、隨 file 存活；路徑緩衝按報的長度加終止符分配。
    unsafe {
        let mut key = std::ptr::null_mut();
        let mut size = 0u32;
        file.GetReferenceKey(&mut key, &mut size).ok()?;
        let loader: IDWriteLocalFontFileLoader = file.GetLoader().ok()?.cast().ok()?;
        let len = loader.GetFilePathLengthFromKey(key, size).ok()?;
        let mut buffer = vec![0u16; len as usize + 1];
        loader.GetFilePathFromKey(key, size, &mut buffer).ok()?;
        buffer.truncate(len as usize);
        Some(PathBuf::from(String::from_utf16_lossy(&buffer)))
    }
}

/// 系統裡全部字族名（英文名，沒有英文名取第一個），按名字排序。
pub fn families() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let Some(collection) = system_collection() else {
        return names;
    };
    // SAFETY: 只讀查詢。
    unsafe {
        for i in 0..collection.GetFontFamilyCount() {
            let name = collection
                .GetFontFamily(i)
                .ok()
                .and_then(|family| family.GetFamilyNames().ok())
                .and_then(|strings| localized(&strings));
            if let Some(name) = name
                && !names.contains(&name)
            {
                names.push(name);
            }
        }
    }
    names.sort_by_key(|name| name.to_lowercase());
    names
}

/// 本地化字串裡的英文名，沒有就取第 0 條。
fn localized(strings: &IDWriteLocalizedStrings) -> Option<String> {
    // SAFETY: 只讀查詢；緩衝按報的長度加終止符分配。
    unsafe {
        let mut index = 0u32;
        let mut exists = BOOL(0);
        let _ = strings.FindLocaleName(&HSTRING::from("en-us"), &mut index, &mut exists);
        if !exists.as_bool() {
            index = 0;
        }
        let len = strings.GetStringLength(index).ok()?;
        let mut buffer = vec![0u16; len as usize + 1];
        strings.GetString(index, &mut buffer).ok()?;
        buffer.truncate(len as usize);
        Some(String::from_utf16_lossy(&buffer))
    }
}
