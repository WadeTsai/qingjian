//! `cfg(windows)`：TSF 文本服務的 COM 外殼。DLL 入口在這裡，類廠在 [`factory`]，文本服務物件在 [`service`]，
//! 登錄檔 / TSF profile 在 [`registry`]。
//!
//! - [`key`]：按鍵翻譯、單擊中英切換鍵判定、翻譯快捷鍵的保留鍵。
//! - [`mode`]：中 / 英模式的指示（轉換模式 compartment、語言欄按鈕）與反向同步（工作列點選、系統熱鍵關掉輸入法）。
//! - [`profile`]：登記的兩個語言 profile（「中文(台灣)」的青簡與青簡注音，都出繁體）與切換通知。
//! - [`edit`]：編輯會話（寫組句 / 讀選區 / 讀前文與輸入範圍）與候選視窗錨點；[`context`]：上下文的鍵盤停用開關（密碼框）。
//! - [`composition`]：組句 preedit；[`display_attribute`]：組句內聯下劃線；[`poll`]：輪詢定時器。
#![allow(non_snake_case)] // 匯出的 Dll* 入口按 COM 約定命名

pub(crate) mod composition;
pub(crate) mod context;
pub(crate) mod display_attribute;
pub(crate) mod edit;
pub(crate) mod factory;
pub(crate) mod focus;
pub(crate) mod key;
pub(crate) mod log;
pub(crate) mod mode;
pub(crate) mod poll;
pub(crate) mod profile;
pub(crate) mod registry;
pub(crate) mod service;
pub(crate) mod window_class;

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicIsize, AtomicPtr, Ordering};

use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, E_FAIL, HINSTANCE, HMODULE, S_FALSE, S_OK,
};
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::core::{BOOL, GUID, HRESULT, HSTRING, Interface};

use qingjian_platform::protocol::SessionId;

/// 文本服務的 CLSID。登錄檔 InprocServer32、TSF profile、[`DllGetClassObject`] 都認它。
pub(crate) const CLSID_QINGJIAN: GUID = GUID::from_u128(0x4fdca82d_e923_49bf_9e75_bb906b93b8bb);

/// [`CLSID_QINGJIAN`] 的登錄檔字串形式，兩者必須同步改。
pub(crate) const CLSID_QINGJIAN_STR: &str = "{4FDCA82D-E923-49BF-9E75-BB906B93B8BB}";

/// 文本服務（CLSID 登錄檔項）的名字。各語言下顯示的名字跟 profile 走，見 [`profile::language`]。
pub(crate) const SERVICE_DESCRIPTION: &str = "青簡";

/// 存活的 COM 物件 + LockServer 計數，[`DllCanUnloadNow`] 據它判斷能否解除安裝。
static DLL_REFERENCES: AtomicIsize = AtomicIsize::new(0);

/// 本 DLL 的模組控制代碼，[`DllMain`] 載入時記下。
static DLL_MODULE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

pub(crate) fn lock_module() {
    DLL_REFERENCES.fetch_add(1, Ordering::SeqCst);
}

pub(crate) fn unlock_module() {
    DLL_REFERENCES.fetch_sub(1, Ordering::SeqCst);
}

/// 本 DLL 的例項控制代碼（註冊視窗類 / 建視窗用）。
pub(crate) fn dll_instance() -> HINSTANCE {
    HINSTANCE(DLL_MODULE.load(Ordering::SeqCst))
}

/// 本 DLL 在磁碟上的完整路徑，註冊 InprocServer32 用。
pub(crate) fn module_path() -> windows::core::Result<HSTRING> {
    let module = HMODULE(DLL_MODULE.load(Ordering::SeqCst));
    let mut buf = [0u16; 260];
    let len = unsafe { GetModuleFileNameW(Some(module), &mut buf) } as usize;
    if len == 0 || len >= buf.len() {
        return Err(E_FAIL.into());
    }
    Ok(HSTRING::from_wide(&buf[..len]))
}

/// 本執行緒的會話號：執行緒 id，全系統唯一（TSF 的 client id 各程序都是同樣那幾個值，會撞號）。
pub(crate) fn session_id() -> SessionId {
    SessionId(u64::from(unsafe { GetCurrentThreadId() }))
}

/// 宿主應用的 exe 檔名（`Code.exe`），開會話時報給 Server（對應 macOS 端的 bundle identifier）。
pub(crate) fn host_app_name() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let name = exe.file_name()?.to_string_lossy().into_owned();
    (!name.is_empty()).then_some(name)
}

#[unsafe(no_mangle)]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_FAIL;
    }
    if unsafe { *rclsid } != CLSID_QINGJIAN {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = factory::ClassFactory.into();
    unsafe { factory.query(riid, ppv) }
}

#[unsafe(no_mangle)]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    if DLL_REFERENCES.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllRegisterServer() -> HRESULT {
    match registry::register() {
        Ok(()) => S_OK,
        Err(error) => {
            log::log(&format!("DllRegisterServer 失败: {error}"));
            error.code()
        }
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllUnregisterServer() -> HRESULT {
    match registry::unregister() {
        Ok(()) => S_OK,
        Err(error) => error.code(),
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllMain(hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DLL_MODULE.store(hinst.0, Ordering::SeqCst);
        log::log("DllMain: DLL_PROCESS_ATTACH");
    }
    true.into()
}
