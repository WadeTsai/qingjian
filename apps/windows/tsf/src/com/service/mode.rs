//! 中 / 英模式：全域性一份，存在 Server 那邊，所有應用共用。
//! 使用者在這裡切了就報給 Server（[`TextService_Impl::set_english_mode`]）；啟用、得到焦點和每隔幾拍的輪詢
//! 向 Server 取當前模式跟上（[`TextService_Impl::sync_mode_from_server`]）。切模式先把組著的內容落定；
//! 指示器走語言欄按鈕 + 轉換模式 compartment；使用者點工作列中 / 英由 compartment 回撥反向同步。
//! 切換鍵與內建英文模式開關由 Server 經協議下發（[`TextService_Impl::apply_input_settings`]），DLL 不讀配置檔案。

use std::time::Instant;

use windows::Win32::UI::TextServices::{ITfKeystrokeMgr, ITfLangBarItemMgr};
use windows::core::Interface;

use qingjian_platform::SwitchKeys;
use qingjian_platform::protocol::InputSettings;

use super::TextService_Impl;
use crate::com::key::preserved;
use crate::com::log::log;
use crate::com::mode::{self, ModeButton, sink};

/// 啟用後多久內忽略系統寫回的轉換模式：msctf 在 TIP 啟用後約 200–300 ms 會把 profile 存的
/// 轉換模式寫回 compartment，不忽略的話它會被當成使用者點了工作列、改掉全域性模式。
pub(super) const CONVERSION_RESTORE_GUARD: std::time::Duration =
    std::time::Duration::from_millis(1200);

impl TextService_Impl {
    /// 應用中英模式的兩項設定：啟用時與配置變更時都走這裡。
    ///
    /// 內建英文模式開關在執行中翻轉時，語言欄的中 / 英按鈕與轉換模式回撥跟著登記 / 撤掉（啟用時由 `Activate`
    /// 自己按開關登記，這裡只管啟用之後的變化），設定視窗改完不用切走再切回輸入法。
    pub(super) fn apply_mode_settings(&self, english_mode: bool, switch_keys: SwitchKeys) {
        let was_enabled = self.mode_state.enabled();
        self.mode_state.set_settings(english_mode, switch_keys);
        // 關掉內建英文模式時立刻回中文，別停在一個再也切不回去的英文狀態（Server 那份也跟著回中文）。
        if !english_mode && self.mode_state.english() {
            self.mode_state.set_english(false);
            self.refresh_mode_indicator();
        }
        if was_enabled != english_mode && self.is_active() {
            if english_mode {
                log("打開了內建英文模式：登記中 / 英按鈕");
                self.add_lang_bar_item();
                self.advise_mode_sinks();
            } else {
                log("關掉了內建英文模式：撤掉中 / 英按鈕");
                self.unadvise_mode_sinks();
                self.remove_lang_bar_item();
            }
        }
        self.sync_switch_preserved_key(switch_keys.ctrl_alt_space);
    }

    /// `Activate` 是否已經走完（[`super::ACTIVE`] 在它末尾才設）。
    fn is_active(&self) -> bool {
        super::ACTIVE.with(|active| active.borrow().is_some())
    }

    /// 應用 Server 下發的按鍵行為設定：`OpenSession` 的回包給一次，之後每一拍 `SyncMode` 也都帶著。
    /// 值沒變就什麼都不做，所以設定視窗改完在下一拍（約 320 ms）生效，不用切走再切回輸入法。
    pub(super) fn apply_input_settings(&self, input: InputSettings) {
        if self.input_settings.get() == Some(input) {
            return;
        }
        self.input_settings.set(Some(input));
        log(&format!(
            "按鍵行為設定：中英切換鍵 {}，內建英文模式 {}，Shift 字母進組句 {}，英文候選 {}",
            input.switch_mode.describe(),
            input.english_mode,
            input.shift_letter_compose,
            input.english_candidates
        ));
        self.apply_mode_settings(input.english_mode, input.switch_mode);
    }

    /// Ctrl + Alt + Space 是組合鍵、走 TSF 保留鍵（與「翻譯選中文字」同一套）；沒勾就撤掉登記，免得白佔著。
    fn sync_switch_preserved_key(&self, want: bool) {
        if want == self.switch_preserved.get() {
            return;
        }
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>() else {
            return;
        };
        if want {
            match preserved::register_switch_mode(&keystroke, self.client_id.get()) {
                Ok(()) => {
                    self.switch_preserved.set(true);
                    log("中英切換鍵 Ctrl + Alt + Space 已登記為保留鍵");
                }
                Err(error) => log(&format!("登記 Ctrl + Alt + Space 切換鍵失敗: {error}")),
            }
        } else {
            preserved::unregister_switch_mode(&keystroke);
            self.switch_preserved.set(false);
        }
    }

    /// 停用時撤掉 Ctrl + Alt + Space 的保留鍵登記。
    pub(super) fn drop_switch_preserved_key(&self, keystroke: &ITfKeystrokeMgr) {
        if self.switch_preserved.replace(false) {
            preserved::unregister_switch_mode(keystroke);
        }
    }

    /// 使用者在這個應用裡切了模式（切換鍵、語言欄按鈕、右鍵選單）：改狀態、刷指示器，報給 Server 成為全域性模式。
    pub(super) fn set_english_mode(&self, english: bool) {
        if self.switch_mode(english) {
            self.refresh_mode_indicator();
            self.report_mode();
        }
    }

    /// 跟上 Server 的全域性模式（別的應用切過、點了懸浮狀態條）：改狀態、刷指示器，不再回報。
    pub(super) fn adopt_mode(&self, english: bool) {
        if english != self.mode_state.english() && self.switch_mode(english) {
            self.refresh_mode_indicator();
        }
    }

    /// 向 Server 取一次全域性模式跟上，順路取回按鍵行為設定與選單狀態；沒連著就什麼都不做。
    pub(super) fn sync_mode_from_server(&self) {
        let reply = self
            .engine
            .borrow_mut()
            .as_mut()
            .map(|client| client.sync_mode());
        match reply {
            Some(Ok(reply)) => {
                self.apply_input_settings(reply.input);
                self.indicator_state.set(reply.indicator);
                if let Some(english) = reply.english {
                    self.adopt_mode(english);
                }
            }
            Some(Err(error)) => {
                log(&format!("同步中英模式失敗，斷開，下一鍵重連: {error}"));
                self.disconnect();
            }
            None => {}
        }
    }

    /// 使用者點了工作列中 / 英、按了系統 Ctrl + Space：只更新按鈕，不回寫 compartment（在它自己的 `OnChange` 裡寫會報
    /// 0x8000FFFF），報給 Server。
    fn follow_system_mode(&self, english: bool) {
        if self.switch_mode(english) {
            self.mode_state.notify();
            self.report_mode();
        }
    }

    /// 兩條路共用：先把組著的內容原樣落定，再改狀態。返回是否真的改了。
    ///
    /// 配置關掉了內建英文模式時什麼都不做——切換鍵、語言欄按鈕、懸浮狀態條、工作列轉換模式四條入口
    /// 都匯到這裡，一處攔住就再也進不了英文模式（見 issue #81）。
    fn switch_mode(&self, english: bool) -> bool {
        if !self.mode_state.enabled() {
            if english {
                log("內建英文模式已關閉，忽略切到英文");
            }
            return false;
        }
        self.commit_pending();
        self.mode_state.set_english(english);
        log(if english {
            "切到英文模式"
        } else {
            "切到中文模式"
        });
        true
    }

    /// 語言欄按鈕換圖示，寫轉換模式與輸入法開關兩條 compartment：開關跟著模式走（中文開、英文關），
    /// 系統的 Ctrl + Space 翻的就是它，對上了才能一按就切。
    pub(super) fn refresh_mode_indicator(&self) {
        self.mode_state.notify();
        let english = self.mode_state.english();
        if let Some(thread_mgr) = self.thread_mgr.borrow().as_ref() {
            mode::set_indicator(thread_mgr, self.client_id.get(), english);
            mode::set_keyboard_open(thread_mgr, self.client_id.get(), !english);
        }
    }

    /// 本執行緒得到 / 失去鍵盤焦點。得到：補連線，跟上全域性模式（別的應用裡可能剛切過）。
    pub(super) fn set_thread_focus(&self, foreground: bool) {
        if self.shared.foreground() == foreground {
            return;
        }
        self.shared.set_foreground(foreground);
        if foreground {
            self.ensure_connected();
            self.sync_mode_from_server();
        }
    }

    /// 把使用者切出的模式報給 Server，成為全域性模式。
    pub(super) fn report_mode(&self) {
        let english = self.mode_state.english();
        if let Some(client) = self.engine.borrow_mut().as_mut()
            && let Err(error) = client.mode_changed(english)
        {
            log(&format!("上報中英模式失敗: {error}"));
        }
    }

    /// 掛上「轉換模式」與「輸入法開 / 關」的回撥。
    pub(super) fn advise_mode_sinks(&self) {
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        *self.mode_sinks.borrow_mut() = sink::advise(&thread_mgr);
    }

    pub(super) fn unadvise_mode_sinks(&self) {
        sink::unadvise(std::mem::take(&mut *self.mode_sinks.borrow_mut()));
    }

    /// 使用者在工作列點了中 / 英：讀回 `NATIVE` 位，與當前不同才跟著切（相同是自己那次寫觸發的，防迴環）。
    ///
    /// 啟用後的最初一瞬不算：那時 msctf 在把 profile 存的轉換模式寫回來，不是使用者操作。
    pub(super) fn sync_from_conversion_mode(&self) {
        if let Some(until) = self.conversion_guard_until.get()
            && Instant::now() < until
        {
            log("啟用後忽略一次系統寫回的轉換模式（msctf 的 profile 恢復，不是使用者操作）");
            return;
        }
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let Ok(compartment) = mode::conversion_compartment(&thread_mgr) else {
            return;
        };
        let english = mode::is_english(&compartment);
        if english != self.mode_state.english() {
            log(&format!(
                "轉換模式變了（工作列 / 系統快捷鍵），english={english}"
            ));
            self.follow_system_mode(english);
        }
    }

    /// 系統的「輸入法/非輸入法切換」（預設 Ctrl + Space）翻了輸入法開關：關 = 英文、開 = 中文，與微軟拼音一致。
    /// 這個熱鍵的 Space 被系統截走，按著的 Ctrl 抬起時別再當成單擊。
    pub(super) fn sync_from_keyboard_open(&self) {
        self.key_tap.cancel();
        let Some(thread_mgr) = self.thread_mgr.borrow().clone() else {
            return;
        };
        let Ok(compartment) = mode::openclose_compartment(&thread_mgr) else {
            return;
        };
        let english = !mode::is_keyboard_open(&compartment);
        if english != self.mode_state.english() {
            log(&format!(
                "輸入法開關變了（系統 Ctrl + Space），english={english}"
            ));
            self.follow_system_mode(english);
        }
    }

    pub(super) fn add_lang_bar_item(&self) {
        let button = ModeButton::create(self.mode_state.clone());
        if let Some(thread_mgr) = self.thread_mgr.borrow().as_ref() {
            match thread_mgr.cast::<ITfLangBarItemMgr>() {
                Ok(mgr) => {
                    if let Err(error) = unsafe { mgr.AddItem(&button) } {
                        log(&format!("登記中英指示器失敗: {error}"));
                    }
                }
                Err(error) => log(&format!("取語言欄管理器失敗: {error}")),
            }
        }
        *self.mode_button.borrow_mut() = Some(button);
    }

    pub(super) fn remove_lang_bar_item(&self) {
        if let Some(button) = self.mode_button.borrow_mut().take()
            && let Some(thread_mgr) = self.thread_mgr.borrow().as_ref()
            && let Ok(mgr) = thread_mgr.cast::<ITfLangBarItemMgr>()
        {
            let _ = unsafe { mgr.RemoveItem(&button) };
        }
    }
}
