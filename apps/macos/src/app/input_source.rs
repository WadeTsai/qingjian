//! 把 `.app` 註冊成系統輸入源、啟用並切成當前輸入源：`qingjian-macos --register`（pkg 的 postinstall 以登入使用者身份調）。
//! 走 Carbon 的 Text Input Source Services，沒有 Cocoa 替代品；註冊時序與快取的坑見 docs/design/architecture.md。

use std::ffi::c_void;
use std::path::Path;
use std::ptr::NonNull;

use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFRetained, CFString, CFURL};
use objc2_foundation::{NSArray, NSBundle, NSDictionary, NSString};

/// TIS 的輸入源控制代碼（不透明）。
#[repr(C)]
struct TISInputSource {
    _private: [u8; 0],
}

// HIToolbox 在 Carbon 傘框架裡；CoreFoundation 由 objc2-core-foundation 連結
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    /// 把一個輸入法 bundle 登記到系統的輸入源列表裡（已登記的再登記無害）。
    fn TISRegisterInputSource(location: NonNull<CFURL>) -> i32;

    /// 按屬性字典篩輸入源；`include_all_installed` 為真時連沒啟用的也列出來。返回的陣列歸呼叫方釋放。
    fn TISCreateInputSourceList(
        properties: *const c_void,
        include_all_installed: u8,
    ) -> *mut CFArray;

    /// 把輸入源加進使用者的輸入法選單。
    fn TISEnableInputSource(source: *const TISInputSource) -> i32;

    /// 把輸入源切成當前輸入源（裝完就能打字，不用再去選單裡挑）。
    fn TISSelectInputSource(source: *const TISInputSource) -> i32;

    /// 讀輸入源的一個屬性；返回值歸系統，不釋放。
    fn TISGetInputSourceProperty(
        source: *const TISInputSource,
        key: NonNull<CFString>,
    ) -> *const c_void;

    /// 屬性鍵：輸入源 ID（`Info.plist` 的 `TISInputSourceID`）。
    static kTISPropertyInputSourceID: NonNull<CFString>;

    /// 屬性鍵：是否已啟用（CFBoolean）。
    static kTISPropertyInputSourceIsEnabled: NonNull<CFString>;
}

/// 註冊當前程序所在的 `.app`、啟用並切成當前輸入源。啟用成功返回 `Ok(是否也切成了當前)`，失敗帶一句能打到安裝日誌裡的說明。
pub fn register_main_bundle() -> Result<bool, String> {
    let bundle = NSBundle::mainBundle();
    let path = bundle.bundlePath().to_string();
    if !path.ends_with(".app") {
        return Err(format!("不是从 .app 里运行的：{path}"));
    }
    let source_id = enabled_source_id(&bundle);
    register_and_enable(Path::new(&path), &source_id)
}

/// 要啟用的輸入源 ID：有輸入模式就是第一個可見模式，否則是頂層 `TISInputSourceID`。
fn enabled_source_id(bundle: &NSBundle) -> String {
    let first_mode = bundle
        .objectForInfoDictionaryKey(&NSString::from_str("ComponentInputModeDict"))
        .and_then(|value| value.downcast::<NSDictionary>().ok())
        .and_then(|modes| {
            modes.objectForKey(&*NSString::from_str("tsVisibleInputModeOrderedArrayKey"))
        })
        .and_then(|value| value.downcast::<NSArray>().ok())
        .and_then(|order| order.firstObject())
        .and_then(|id| id.downcast::<NSString>().ok())
        .map(|id| id.to_string());
    first_mode.unwrap_or_else(|| {
        bundle
            .objectForInfoDictionaryKey(&NSString::from_str("TISInputSourceID"))
            .and_then(|value| value.downcast::<NSString>().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| super::bundle::DEFAULT_IDENTIFIER.to_owned())
    })
}

/// 註冊 `app`，啟用 ID 為 `source_id` 的輸入源並切成當前。返回是否也切成了當前（切換失敗不算錯）。
///
/// 每一輪：註冊 → 啟用 → 子程序（`--finish-register`）回讀並切換 → 隔 [`CONFIRM`] 再回讀一次確認沒被系統重掃頂掉；
/// 沒啟用就等 [`RETRY_INTERVAL`] 重來，最多 [`ENABLE_TIMEOUT`]。回讀必須在新程序裡做，本程序讀到的是快取。
pub fn register_and_enable(app: &Path, source_id: &str) -> Result<bool, String> {
    let url =
        CFURL::from_file_path(app).ok_or_else(|| format!("路径无法转成 URL：{}", app.display()))?;
    let exe = std::env::current_exe().map_err(|e| format!("找不到自己的可执行文件：{e}"))?;
    let started = std::time::Instant::now();
    loop {
        // SAFETY: url 是有效的 CFURL，函式只讀它。
        let status = unsafe { TISRegisterInputSource(CFRetained::as_ptr(&url)) };
        if status != 0 {
            return Err(format!("TISRegisterInputSource 失败（{status}）"));
        }
        if let Some(sources) = list_sources(source_id).filter(|s| s.count() > 0) {
            for_each_source(&sources, |source| {
                // SAFETY: source 來自還活著的陣列。
                let status = unsafe { TISEnableInputSource(source) };
                if status == 0 {
                    Ok(())
                } else {
                    Err(format!("TISEnableInputSource 失败（{status}）"))
                }
            })?;
            std::thread::sleep(SETTLE);
            let first = finish_in_child(&exe, source_id)?;
            if first != FINISH_NOT_ENABLED {
                std::thread::sleep(CONFIRM);
                let second = finish_in_child(&exe, source_id)?;
                match second {
                    FINISH_SELECTED => return Ok(true),
                    FINISH_ENABLED_ONLY => return Ok(false),
                    _ => eprintln!("输入源启用后又被系统重扫顶掉，重来"),
                }
            }
        }
        if started.elapsed() > ENABLE_TIMEOUT {
            return Err(format!(
                "输入源启用没有生效（等了 {} 秒）：{source_id}",
                ENABLE_TIMEOUT.as_secs()
            ));
        }
        std::thread::sleep(RETRY_INTERVAL);
    }
}

/// 起子程序跑 [`finish_register`]，返回它的退出碼。
fn finish_in_child(exe: &Path, source_id: &str) -> Result<i32, String> {
    std::process::Command::new(exe)
        .arg(FINISH_FLAG)
        .arg(source_id)
        .status()
        .map_err(|e| format!("起不了子进程回读状态：{e}"))
        .map(|status| status.code().unwrap_or(FINISH_NOT_ENABLED))
}

/// `--register` 的收尾，在新程序裡跑（見 [`register_and_enable`]）：ID 為 `source_id` 的輸入源都已啟用就切成當前。
/// 返回程序退出碼：[`FINISH_SELECTED`] / [`FINISH_ENABLED_ONLY`] / [`FINISH_NOT_ENABLED`]。
pub fn finish_register(source_id: &str) -> i32 {
    let Some(sources) = list_sources(source_id).filter(|s| s.count() > 0) else {
        return FINISH_NOT_ENABLED;
    };
    let mut all_enabled = true;
    let mut selected = true;
    let _ = for_each_source(&sources, |source| {
        if !is_enabled(source) {
            all_enabled = false;
            return Ok(());
        }
        // SAFETY: source 來自還活著的陣列。
        let status = unsafe { TISSelectInputSource(source) };
        if status != 0 {
            eprintln!("TISSelectInputSource 失败（{status}），输入源已启用但没切成当前");
            selected = false;
        }
        Ok(())
    });
    match (all_enabled, selected) {
        (false, _) => FINISH_NOT_ENABLED,
        (true, true) => FINISH_SELECTED,
        (true, false) => FINISH_ENABLED_ONLY,
    }
}

/// 子程序的命令列開關：`qingjian-macos --finish-register <輸入源 ID>`。
pub const FINISH_FLAG: &str = "--finish-register";

/// 子程序退出碼：已啟用並切成當前。
pub const FINISH_SELECTED: i32 = 0;

/// 子程序退出碼：沒啟用（要再等）。
pub const FINISH_NOT_ENABLED: i32 = 1;

/// 子程序退出碼：已啟用但切換失敗。
pub const FINISH_ENABLED_ONLY: i32 = 2;

/// 回讀輸入源的啟用狀態。
fn is_enabled(source: *const TISInputSource) -> bool {
    // SAFETY: source 來自系統返回的陣列且陣列還活著；屬性值是 CFBoolean，歸系統所有，只讀不釋放。
    unsafe {
        let value = TISGetInputSourceProperty(source, kTISPropertyInputSourceIsEnabled);
        value
            .cast::<CFBoolean>()
            .as_ref()
            .is_some_and(CFBoolean::value)
    }
}

/// 對列表裡的每個輸入源調一次 `f`，第一個錯誤就返回。
fn for_each_source(
    sources: &CFArray,
    mut f: impl FnMut(*const TISInputSource) -> Result<(), String>,
) -> Result<(), String> {
    for index in 0..sources.count() {
        // SAFETY: index 在範圍內；元素是 TISInputSourceRef，陣列活著它就有效。
        let source = unsafe { sources.value_at_index(index) }.cast::<TISInputSource>();
        f(source)?;
    }
    Ok(())
}

/// 啟用後等系統把新包掃描完再回讀的時間。
const SETTLE: std::time::Duration = std::time::Duration::from_millis(1500);

/// 第一次確認成功後再等多久做第二次確認（系統重掃新包大約在裝完 3–5 秒時把記錄換掉）。
const CONFIRM: std::time::Duration = std::time::Duration::from_secs(3);

/// 兩輪之間的間隔。
const RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

/// 啟用生效的等待上限。
const ENABLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// 列出 ID 為 `source_id` 的輸入源（含未啟用的）；系統沒給陣列時返回 `None`。
fn list_sources(source_id: &str) -> Option<CFRetained<CFArray>> {
    // SAFETY: kTISPropertyInputSourceID 是框架匯出的常量字串。
    let key: &CFString = unsafe { kTISPropertyInputSourceID.as_ref() };
    let value = CFString::from_str(source_id);
    let filter = CFDictionary::<CFString, CFString>::from_slices(&[key], &[&value]);
    // SAFETY: 字典有效；返回值按 Create 規則歸呼叫方，用 from_raw 接管釋放。
    unsafe {
        let raw =
            TISCreateInputSourceList(CFRetained::as_ptr(&filter).as_ptr().cast::<c_void>(), 1);
        NonNull::new(raw).map(|ptr| CFRetained::from_raw(ptr))
    }
}
