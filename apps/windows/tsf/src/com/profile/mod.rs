//! 語言 profile：青簡登記的兩個 profile（簡 / 繁，[`language`]）、查當前啟用的是哪個、以及切換通知。
//!
//! 本執行緒把輸入法切成別的（微軟拼音等）時告訴 Server 收狀態條。
//! 應用退出時只有 `Deactivate`、沒有別的 profile 被啟用，所以狀態條不跟會話開關走。
//! 通知在青簡停用**之後**到來，那時會話連線已關，用一條臨時連線發。
//! 在青簡自己的兩個 profile 之間切換（「中文(簡體)」的青簡 ↔「中文(台灣)」的青簡）TSF 不停用再啟用，
//! 只來這條通知，據它把簡 / 繁報給 Server。
//! sink 掛在執行緒管理器上，啟用一次、之後不撤（執行緒管理器銷燬時自然釋放），重複啟用不再掛。

pub(crate) mod language;

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::TextServices::{
    CLSID_TF_InputProcessorProfiles, ITfActiveLanguageProfileNotifySink,
    ITfActiveLanguageProfileNotifySink_Impl, ITfInputProcessorProfiles, ITfSource, ITfThreadMgr,
};
use windows::core::{BOOL, GUID, Interface, Result, implement};

use qingjian_platform::protocol::SessionId;

use super::log::log;
use crate::client::EngineClient;
use crate::client::pipe::connect_default;

#[implement(ITfActiveLanguageProfileNotifySink)]
struct ProfileSink {
    /// 通知裡帶的會話標識（Server 只當訊號，不查會話）。
    session: SessionId,
}

impl ProfileSink {
    fn new(session: SessionId) -> Self {
        super::lock_module();
        Self { session }
    }
}

impl Drop for ProfileSink {
    fn drop(&mut self) {
        super::unlock_module();
    }
}

impl ITfActiveLanguageProfileNotifySink_Impl for ProfileSink_Impl {
    fn OnActivated(
        &self,
        clsid: *const GUID,
        guidprofile: *const GUID,
        factivated: BOOL,
    ) -> Result<()> {
        if !factivated.as_bool() {
            return Ok(());
        }
        if clsid.is_null() {
            return Ok(());
        }
        if unsafe { *clsid } == super::CLSID_QINGJIAN {
            let traditional =
                !guidprofile.is_null() && language::is_traditional(unsafe { &*guidprofile });
            super::service::on_profile_activated(traditional);
            return Ok(());
        }
        let sent = connect_default()
            .map_err(|e| e.to_string())
            .and_then(|stream| {
                EngineClient::notify_ime_switched(stream, self.session).map_err(|e| e.to_string())
            });
        match sent {
            Ok(()) => log("切到別的輸入法，已通知 Server 收狀態條"),
            Err(error) => log(&format!("切到別的輸入法，通知 Server 失敗: {error}")),
        }
        Ok(())
    }
}

/// 掛上通知，返回 cookie（只用來判斷掛過沒有）。
pub(super) fn advise(thread_mgr: &ITfThreadMgr, session: SessionId) -> Result<u32> {
    let source: ITfSource = thread_mgr.cast()?;
    let sink: ITfActiveLanguageProfileNotifySink = ProfileSink::new(session).into();
    unsafe { source.AdviseSink(&ITfActiveLanguageProfileNotifySink::IID, &sink) }
}

/// 本執行緒當前啟用的是青簡的繁體 profile 嗎；查不到（還沒啟用任何一個）為 `None`。
/// 啟用時 TSF 已把 profile 記為當前，`Activate` 裡就能查。
pub(super) fn active_traditional() -> Option<bool> {
    let created: Result<ITfInputProcessorProfiles> = unsafe {
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)
    };
    let profiles = created.ok()?;
    let mut langid = 0u16;
    let mut guid = GUID::zeroed();
    let active = unsafe {
        profiles.GetActiveLanguageProfile(&super::CLSID_QINGJIAN, &mut langid, &mut guid)
    };
    active.ok()?;
    Some(language::is_traditional(&guid))
}
