//! 往文件寫字：按鍵結果經非同步編輯會話寫上屏文本 + 組句拼音行；失焦 / 停用 / 切模式時讓 Server 交出緩衝區原樣落定；
//! 翻譯選中文字時起只讀會話讀選區。

use windows::Win32::UI::TextServices::ITfContext;
use windows::core::Ref;

use super::TextService_Impl;
use crate::com::edit::{request_selection, request_update};
use crate::com::log::log;

impl TextService_Impl {
    pub(super) fn read_selection(&self, pic: Ref<ITfContext>, request: u64) {
        let Ok(context) = pic.ok() else {
            log("翻譯選中文字：無上下文，取消讀選區");
            return;
        };
        if let Err(error) = request_selection(
            context,
            self.client_id.get(),
            self.engine.clone(),
            self.shared.clone(),
            request,
        ) {
            log(&format!("請求讀選區會話失敗: {error}"));
        }
    }

    /// 失焦 / 停用 / 切模式：讓 Server 交出緩衝區，原樣落進最近收鍵的文件並收掉組句。
    /// 組句已被應用終止的（拼音已是普通文本）只清 Server 不再插。
    pub(super) fn commit_pending(&self) {
        if self.shared.translating() {
            self.shared.set_translating(false);
            self.shared.hide_candidates();
        }
        let stale = self.shared.take_server_stale();
        if !self.shared.composing() && !stale {
            return;
        }
        let text = {
            let mut guard = self.engine.borrow_mut();
            let Some(client) = guard.as_mut() else {
                self.shared.reset();
                return;
            };
            match client.commit() {
                Ok(text) => text,
                Err(error) => {
                    log(&format!("失焦上屏失敗，斷開，下一鍵重連: {error}"));
                    drop(guard);
                    self.disconnect();
                    return;
                }
            }
        };
        if stale {
            return;
        }
        self.shared.end_composing();
        let Some(context) = self.shared.last_context() else {
            log(&format!("失焦上屏沒有上下文，丟棄: {text:?}"));
            self.shared.reset();
            return;
        };
        log(&format!("失焦上屏: {text:?}"));
        let requested = request_update(
            &context,
            self.client_id.get(),
            self.engine.clone(),
            self.shared.clone(),
            text.filter(|t| !t.is_empty()),
            String::new(),
        );
        if let Err(error) = requested {
            log(&format!("失焦上屏的編輯會話沒被受理: {error}"));
            self.shared.reset();
        }
    }

    /// 經非同步編輯會話把上屏文本 + 組句拼音行寫進文件。
    pub(super) fn update_document(
        &self,
        pic: Ref<ITfContext>,
        commit: Option<String>,
        preedit: String,
    ) {
        // 退到最後一個字母時幀已空但組句控制代碼還在，得跑一次把它收掉。
        if commit.is_none()
            && preedit.is_empty()
            && !self.shared.composing()
            && !self.shared.has_composition()
        {
            return;
        }
        let Ok(context) = pic.ok() else {
            log(&format!(
                "無上下文，丟棄更新: commit={commit:?} preedit={preedit:?}"
            ));
            return;
        };
        if let Err(error) = request_update(
            context,
            self.client_id.get(),
            self.engine.clone(),
            self.shared.clone(),
            commit,
            preedit,
        ) {
            log(&format!("請求組句更新失敗: {error}"));
        }
    }
}
