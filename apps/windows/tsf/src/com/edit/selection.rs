//! 「翻譯選中文字」的讀選區會話：Server 收到快捷鍵後請 DLL 讀當前選區，這裡在非同步只讀會話裡取文本與螢幕矩形回給 Server。
//! 讀到非空選區才置 [`Shared::set_translating`]。

use std::rc::Rc;

use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, ITfRange, TF_ES_READ,
};
use windows::core::{Result, implement};

use qingjian_platform::protocol::ScreenRect;

use super::anchor::{anchor_rect, mouse_screen_rect, selection_range};
use crate::com::composition::Shared;
use crate::com::log::log;
use crate::com::service::SharedClient;

/// 原文字元上限（與 macOS 殼一致），超過就不譯，避免整篇誤觸。
const MAX_TRANSLATE_CHARS: usize = 500;

#[implement(ITfEditSession)]
pub(crate) struct SelectionSession {
    /// 目標文件上下文。
    context: ITfContext,

    /// 引擎層：把選區文本發給 Server。
    engine: SharedClient,

    /// 讀到非空選區就置「翻譯評審進行中」。
    shared: Rc<Shared>,

    /// 請求標識，回給 Server 對上是哪一次 `RequestSelection`。
    request: u64,
}

impl ITfEditSession_Impl for SelectionSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        let (text, rect) = read_selection(&self.context, ec);
        let has_text = !text.trim().is_empty();
        // 發成功再置本地評審態，免得卡在無視窗的評審裡。
        if let Ok(mut guard) = self.engine.try_borrow_mut()
            && let Some(client) = guard.as_mut()
        {
            match client.selection(self.request, text, rect) {
                Ok(_) if has_text => self.shared.set_translating(true),
                Ok(_) => {}
                Err(error) => log(&format!("回選區給 Server 失敗: {error}")),
            }
        }
        Ok(())
    }
}

/// 請求一個非同步只讀會話讀選區。`Ok` 只說明已受理。
pub(crate) fn request_selection(
    context: &ITfContext,
    client_id: u32,
    engine: SharedClient,
    shared: Rc<Shared>,
    request: u64,
) -> Result<()> {
    let session = SelectionSession {
        context: context.clone(),
        engine,
        shared,
        request,
    };
    super::update::request(context, client_id, session.into(), TF_ES_READ)
}

/// 沒有選區 / 讀不到 / 過長時文本為空串（矩形退到滑鼠處）。
fn read_selection(context: &ITfContext, ec: u32) -> (String, ScreenRect) {
    let Some(range) = selection_range(context, ec) else {
        return (String::new(), mouse_screen_rect());
    };
    let rect = anchor_rect(context, ec, &range);
    (range_text(&range, ec), rect)
}

fn range_text(range: &ITfRange, ec: u32) -> String {
    let mut buf = [0u16; MAX_TRANSLATE_CHARS + 1];
    let mut fetched = 0u32;
    if unsafe { range.GetText(ec, 0, &mut buf, &mut fetched) }.is_err() {
        return String::new();
    }
    let count = fetched as usize;
    if count == 0 || count > MAX_TRANSLATE_CHARS {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..count])
}
