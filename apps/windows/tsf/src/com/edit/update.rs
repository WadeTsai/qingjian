//! 編輯會話：TSF 不允許直接改文件，要經 `RequestEditSession` 申請，在回撥裡拿著 edit cookie 寫。
//!
//! 必須用非同步會話（不帶 `TF_ES_SYNC`）：沉浸式應用（Win11 新記事本等）的文本存區隔著程序邊界，
//! 同步讀寫會讓 `textinputframework.dll` 訪問違例把宿主整個搞崩。回撥可能在 `OnKeyDown` 返回之後才跑。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use windows::Win32::Foundation::E_FAIL;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, TF_CONTEXT_EDIT_CONTEXT_FLAGS, TF_ES_READWRITE,
};
use windows::core::{Error, Result, implement};

use crate::com::composition::{Shared, apply};
use crate::com::log::log;
use crate::com::service::SharedClient;

/// 一次性的讀寫會話：把本次按鍵的組句更新寫進 `context`。
#[implement(ITfEditSession)]
pub(crate) struct UpdateSession {
    /// 目標文件上下文。
    context: ITfContext,

    /// 引擎層：量到游標矩形後報給 Server 擺候選視窗。
    engine: SharedClient,

    /// 組句狀態。
    shared: Rc<Shared>,

    /// 本次要落定上屏的文本。
    commit: Option<String>,

    /// 本次組句拼音行；空串表示收起組句。
    preedit: String,
}

impl ITfEditSession_Impl for UpdateSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        // 從框架的 C++ 調進來：panic 不能越過 FFI。
        let result = catch_unwind(AssertUnwindSafe(|| {
            apply(
                &self.shared,
                &self.engine,
                &self.context,
                ec,
                self.commit.as_deref(),
                &self.preedit,
            )
        }));
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                log(&format!("組句更新失敗: {error}"));
                Err(error)
            }
            Err(_) => {
                log("組句更新回撥 panic（已兜住）");
                Err(Error::from(E_FAIL))
            }
        }
    }
}

/// 請求一個非同步讀寫會話。`Ok` 只說明已受理，寫入結果在回撥裡記日誌。
pub(crate) fn request_update(
    context: &ITfContext,
    client_id: u32,
    engine: SharedClient,
    shared: Rc<Shared>,
    commit: Option<String>,
    preedit: String,
) -> Result<()> {
    let session = UpdateSession {
        context: context.clone(),
        engine,
        shared,
        commit,
        preedit,
    };
    request(context, client_id, session.into(), TF_ES_READWRITE)
}

/// 非同步提交一個編輯會話；會話物件由框架持有到回撥跑完。
pub(super) fn request(
    context: &ITfContext,
    client_id: u32,
    session: ITfEditSession,
    flags: TF_CONTEXT_EDIT_CONTEXT_FLAGS,
) -> Result<()> {
    unsafe { context.RequestEditSession(client_id, &session, flags)? }.ok()
}
