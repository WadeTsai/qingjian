//! `ITfTextInputProcessor`：啟用時掛擊鍵 sink、登記翻譯保留鍵、連 Server、起輪詢定時器、掛 profile /
//! 模式 compartment 回撥、登記語言欄按鈕；停用按相反順序撤掉，敲了一半的拼音先原樣落定。

use std::time::Instant;

use windows::Win32::UI::TextServices::{
    ITfKeyEventSink, ITfKeystrokeMgr, ITfTextInputProcessor_Impl, ITfThreadMgr,
};
use windows::core::{IUnknownImpl, Interface, Ref, Result};

use qingjian_platform::protocol::InputSettings;

use super::mode::CONVERSION_RESTORE_GUARD;
use super::{ACTIVE, TextService_Impl};
use crate::com::focus;
use crate::com::key::preserved;
use crate::com::log::log;
use crate::com::poll::PollTimer;
use crate::com::profile;

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<ITfThreadMgr>, tid: u32) -> Result<()> {
        let thread_mgr = ptim.ok()?.clone();
        let keystroke: ITfKeystrokeMgr = thread_mgr.cast()?;
        let sink: ITfKeyEventSink = self.to_interface();
        unsafe { keystroke.AdviseKeyEventSink(tid, &sink, true)? };
        let combo = preserved::load_combo();
        match preserved::register(&keystroke, tid, combo) {
            Ok(()) => {
                self.translate_combo.set(Some(combo));
                log(&format!("翻譯選中文字快捷鍵已登記為保留鍵: {combo}"));
            }
            Err(error) => log(&format!("登記翻譯快捷鍵失敗: {error}")),
        }

        self.client_id.set(tid);
        // 連不上 Server、沒定時器都不致命。
        match PollTimer::new(self.engine.clone(), self.shared.clone(), tid) {
            Ok(timer) => *self.poll_timer.borrow_mut() = Some(timer),
            Err(error) => log(&format!("掛雲聯想輪詢定時器失敗: {error}")),
        }

        if self.profile_cookie.get().is_none() {
            match profile::advise(&thread_mgr, crate::com::session_id()) {
                Ok(cookie) => self.profile_cookie.set(Some(cookie)),
                Err(error) => log(&format!("監聽輸入法切換失敗: {error}")),
            }
        }
        match focus::advise(&thread_mgr) {
            Ok(advice) => *self.focus_sink.borrow_mut() = Some(advice),
            Err(error) => log(&format!("監聽執行緒焦點失敗: {error}")),
        }
        // 啟用時執行緒已有焦點的話，不會再來一次「得到焦點」。
        let focused = unsafe { thread_mgr.IsThreadFocus() }.is_ok_and(|focus| focus.as_bool());
        *self.thread_mgr.borrow_mut() = Some(thread_mgr);
        // 連 Server：它隨 `OpenSession` 的回包把按鍵行為設定帶下來，就地應用。那兩個值在按鍵到達之前
        // 就要有，而且應用時要登記 Ctrl + Alt + Space 保留鍵，所以這一步必須排在 `thread_mgr` 就緒之後。
        self.connect();
        // 只有**沒連上** Server 時才用預設值把模式狀態建起來。連上了的話 `connect` 已經應用過 Server 下發的
        // 真實值，這裡再應用一次預設值會把它蓋掉（要等下一拍 `SyncMode` 才改回來 —— 關掉內建英文模式的人
        // 每次啟用都會先登記上中 / 英按鈕，之後也不會撤）。
        if self.engine.borrow().is_none() {
            self.apply_input_settings(InputSettings::default());
        }
        // 模式是全域性的：跟上 Server 那份（沒連上就先中文）。msctf 啟用後還會把上次的轉換模式寫回來，
        // 那不是使用者操作，這段窗口裡忽略它（見 `sync_from_conversion_mode`）。
        self.conversion_guard_until
            .set(Some(Instant::now() + CONVERSION_RESTORE_GUARD));
        self.mode_state.set_english(false);
        if focused {
            self.shared.set_foreground(true);
        }
        self.sync_mode_from_server();
        // 順帶把輸入法開關對上模式：它在新執行緒裡預設是關，不對上的話中文模式下第一次按系統 Ctrl + Space 會白按。
        self.refresh_mode_indicator();
        if self.mode_state.enabled() {
            self.add_lang_bar_item();
            // 放在初始寫指示器 / 開關之後，別被自己那次寫觸發。
            self.advise_mode_sinks();
        } else {
            log("配置關掉了內建英文模式：不登記中 / 英按鈕，固定中文模式");
        }
        ACTIVE.with(|active| *active.borrow_mut() = Some(self.to_object()));
        log(&format!("青簡 TSF 已啟用 tid={tid}"));
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        // 先撤回調，之後不再有回撥碰本服務。
        ACTIVE.with(|active| active.borrow_mut().take());
        self.unadvise_mode_sinks();
        if let Some((source, cookie)) = self.focus_sink.borrow_mut().take() {
            focus::unadvise(&source, cookie);
        }
        self.remove_lang_bar_item();
        self.poll_timer.borrow_mut().take();
        // 切走輸入法時敲了一半的拼音原樣落定，再關會話。
        self.commit_pending();
        if let Some(thread_mgr) = self.thread_mgr.borrow_mut().take()
            && let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>()
        {
            self.drop_switch_preserved_key(&keystroke);
            if let Some(combo) = self.translate_combo.take() {
                preserved::unregister(&keystroke, combo);
            }
            let _ = unsafe { keystroke.UnadviseKeyEventSink(self.client_id.get()) };
        }
        if let Some(client) = self.engine.borrow_mut().take() {
            let _ = client.close();
        }
        self.profile_flags.set(None);
        self.shared.reset();
        self.shared.take_server_stale();
        self.shared.set_foreground(false);
        log("青簡 TSF 已停用");
        Ok(())
    }
}
