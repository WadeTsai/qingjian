//! 執行緒焦點通知：本執行緒得到鍵盤焦點時 service 向 Server 取一次全域性中英模式（別的應用裡可能剛切過）。
//! 切視窗時 `ITfKeyEventSink::OnSetFocus` 不觸發，跟著視窗焦點走的是這條。

use windows::Win32::UI::TextServices::{
    ITfSource, ITfThreadFocusSink, ITfThreadFocusSink_Impl, ITfThreadMgr,
};
use windows::core::{Interface, Result, implement};

#[implement(ITfThreadFocusSink)]
struct ThreadFocusSink;

impl ITfThreadFocusSink_Impl for ThreadFocusSink_Impl {
    fn OnSetThreadFocus(&self) -> Result<()> {
        super::service::on_thread_focus(true);
        Ok(())
    }

    fn OnKillThreadFocus(&self) -> Result<()> {
        super::service::on_thread_focus(false);
        Ok(())
    }
}

/// 掛上通知，返回 `(source, cookie)` 供 [`unadvise`]。
pub(super) fn advise(thread_mgr: &ITfThreadMgr) -> Result<(ITfSource, u32)> {
    let source: ITfSource = thread_mgr.cast()?;
    let sink: ITfThreadFocusSink = ThreadFocusSink.into();
    let cookie = unsafe { source.AdviseSink(&ITfThreadFocusSink::IID, &sink)? };
    Ok((source, cookie))
}

pub(super) fn unadvise(source: &ITfSource, cookie: u32) {
    let _ = unsafe { source.UnadviseSink(cookie) };
}
