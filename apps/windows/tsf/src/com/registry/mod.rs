//! 自注冊：寫 COM 的 InprocServer32，經 `ITfInputProcessorProfiles` / `ITfCategoryMgr` 把青簡登記成鍵盤類文本服務，
//! 語言 profile 只在「中文(台灣)」下登記兩個（拼音與注音，見 [`crate::com::profile::language`]）；
//! 早期版本在「中文(簡體)」下登記過的那個，註冊時先拿掉再登記新的，升級後列表裡不再有簡體的青簡。
//! 寫的是 `HKEY_CLASSES_ROOT`，所以 regsvr32 要管理員。輸入法圖示檔案在 [`icon`]。

mod icon;

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_TIP_KEYBOARD, GUID_TFCAT_TIPCAP_COMLESS, GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT, GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIPCAP_UIELEMENTENABLED, ITfCategoryMgr,
    ITfInputProcessorProfiles,
};
use windows::core::{GUID, Result};
use windows_registry::CLASSES_ROOT;

use crate::com::profile::language::{LEGACY_PROFILES, PROFILES};
use crate::com::{CLSID_QINGJIAN, CLSID_QINGJIAN_STR, SERVICE_DESCRIPTION};

/// 除「鍵盤類 TIP」外還要宣告沉浸式 / 系統托盤等能力，否則 Win10/11 的輸入切換器會把它過濾掉（表現為「裝上又消失」）。
const CATEGORIES: &[GUID] = &[
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_COMLESS,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
];

fn clsid_key() -> String {
    format!("CLSID\\{CLSID_QINGJIAN_STR}")
}

pub(crate) fn register() -> Result<()> {
    let module = crate::com::module_path()?;
    let base = CLASSES_ROOT.create(clsid_key())?;
    base.set_string("", SERVICE_DESCRIPTION)?;
    let inproc = base.create("InprocServer32")?;
    inproc.set_string("", module.to_string())?;
    inproc.set_string("ThreadingModel", "Apartment")?;
    register_profiles()
}

/// 撤銷 [`register`]，盡力而為。
pub(crate) fn unregister() -> Result<()> {
    let _ = unregister_profiles();
    icon::uninstall();
    let _ = CLASSES_ROOT.remove_tree(clsid_key());
    Ok(())
}

fn register_profiles() -> Result<()> {
    com_scope(|| {
        let profiles: ITfInputProcessorProfiles = unsafe {
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?
        };
        // 兩個 profile 共用一張圖示。
        let icon = icon::install()
            .map(|path| wide_z(&path.to_string_lossy()))
            .unwrap_or_else(|| vec![0]);
        unsafe { profiles.Register(&CLSID_QINGJIAN)? };
        // 升級自還登記著簡體 profile 的版本：先把它拿掉（沒登記過就是個空操作，失敗也不擋註冊）。
        for legacy in &LEGACY_PROFILES {
            let _ = unsafe {
                profiles.RemoveLanguageProfile(&CLSID_QINGJIAN, legacy.langid, &legacy.guid)
            };
        }
        for profile in &PROFILES {
            // AddLanguageProfile 按 null 掃描讀字串，不補 0 會多讀相鄰記憶體（曾顯示成「青簡C」）。
            let description = wide_z(profile.description);
            unsafe {
                profiles.AddLanguageProfile(
                    &CLSID_QINGJIAN,
                    profile.langid,
                    &profile.guid,
                    &description,
                    &icon,
                    0,
                )?;
            }
        }
        let category: ITfCategoryMgr =
            unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
        for catid in CATEGORIES {
            unsafe { category.RegisterCategory(&CLSID_QINGJIAN, catid, &CLSID_QINGJIAN)? };
        }
        Ok(())
    })
}

fn unregister_profiles() -> Result<()> {
    com_scope(|| {
        if let Ok(category) = unsafe {
            CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)
        } {
            for catid in CATEGORIES {
                let _ =
                    unsafe { category.UnregisterCategory(&CLSID_QINGJIAN, catid, &CLSID_QINGJIAN) };
            }
        }
        if let Ok(profiles) = unsafe {
            CoCreateInstance::<_, ITfInputProcessorProfiles>(
                &CLSID_TF_InputProcessorProfiles,
                None,
                CLSCTX_INPROC_SERVER,
            )
        } {
            unsafe {
                for profile in PROFILES.iter().chain(LEGACY_PROFILES.iter()) {
                    let _ = profiles.RemoveLanguageProfile(
                        &CLSID_QINGJIAN,
                        profile.langid,
                        &profile.guid,
                    );
                }
                let _ = profiles.Unregister(&CLSID_QINGJIAN);
            }
        }
        Ok(())
    })
}

/// regsvr32 一般已初始化過 COM；已是別的套間模式（`RPC_E_CHANGED_MODE`）就不配對反初始化。
fn com_scope<T>(body: impl FnOnce() -> Result<T>) -> Result<T> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    let result = body();
    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

/// 以 0 結尾的寬字串。
fn wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
