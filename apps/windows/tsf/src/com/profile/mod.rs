//! 语言 profile：青简登记的两个 profile（简 / 繁，[`language`]）、查当前激活的是哪个、以及切换通知。
//!
//! 本线程把输入法切成别的（微软拼音等）时告诉 Server 收状态条。
//! 应用退出时只有 `Deactivate`、没有别的 profile 被激活，所以状态条不跟会话开关走。
//! 通知在青简停用**之后**到来，那时会话连接已关，用一条临时连接发。
//! 在青简自己的两个 profile 之间切换（「中文(简体)」的青简 ↔「中文(台灣)」的青簡）TSF 不停用再激活，
//! 只来这条通知，据它把简 / 繁报给 Server。
//! sink 挂在线程管理器上，激活一次、之后不撤（线程管理器销毁时自然释放），重复激活不再挂。

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
    /// 通知里带的会话标识（Server 只当信号，不查会话）。
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
            Ok(()) => log("切到别的输入法，已通知 Server 收状态条"),
            Err(error) => log(&format!("切到别的输入法，通知 Server 失败: {error}")),
        }
        Ok(())
    }
}

/// 挂上通知，返回 cookie（只用来判断挂过没有）。
pub(super) fn advise(thread_mgr: &ITfThreadMgr, session: SessionId) -> Result<u32> {
    let source: ITfSource = thread_mgr.cast()?;
    let sink: ITfActiveLanguageProfileNotifySink = ProfileSink::new(session).into();
    unsafe { source.AdviseSink(&ITfActiveLanguageProfileNotifySink::IID, &sink) }
}

/// 本线程当前激活的是青简的繁体 profile 吗；查不到（还没激活任何一个）为 `None`。
/// 激活时 TSF 已把 profile 记为当前，`Activate` 里就能查。
pub(super) fn active_traditional() -> Option<bool> {
    let profiles: ITfInputProcessorProfiles = unsafe {
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)
    }
    .ok()?;
    let mut langid = 0u16;
    let mut guid = GUID::zeroed();
    unsafe { profiles.GetActiveLanguageProfile(&super::CLSID_QINGJIAN, &mut langid, &mut guid) }
        .ok()?;
    Some(language::is_traditional(&guid))
}
