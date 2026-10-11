//! `ITfKeyEventSink`：所有鍵先經 `OnTestKeyDown` 判吃不吃（[`TextService_Impl::would_eat`]，與 Router 的分派對齊），
//! 吃的鍵在 `OnKeyDown` 裡轉發給 Server 並按結果更新文件；單擊中英切換鍵（`[shortcut] switch_mode`）的判定與保留鍵命中也在這裡。
//! 上下文禁了鍵盤（密碼框，見 [`context`](crate::com::context)）時沒在組句的鍵一律放行。

use windows::Win32::Foundation::{FALSE, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_CAPITAL;
use windows::Win32::UI::TextServices::{ITfContext, ITfKeyEventSink_Impl};
use windows::core::{BOOL, GUID, Ref, Result};

use qingjian_platform::protocol::{KeyEvent, KeyOutcome};

use super::TextService_Impl;
use super::next::Next;
use crate::client::KeyReply;
use crate::com::composition::preedit_string;
use crate::com::key::event::{digit_key, is_edit, is_letter, is_mode_letter, is_nav, to_key_event};
use crate::com::key::preserved;
use crate::com::log::log;

impl ITfKeyEventSink_Impl for TextService_Impl {
    /// 失焦：把敲了一半的拼音原樣落定（對應 macOS 的 `commitComposition`）。焦點本身交給
    /// [`TextService_Impl::set_thread_focus`]；切視窗時這條回撥不觸發，靠的是 [`crate::com::focus`]。
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        let foreground = fforeground.as_bool();
        if !foreground {
            self.commit_pending();
        }
        self.set_thread_focus(foreground);
        Ok(())
    }

    /// 所有鍵（含之後被吃掉的）都先經過這裡，Shift 單擊的判定放在這一層。
    fn OnTestKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let vk = wparam.0 as u32;
        self.note_key_down(vk, lparam);
        if self.keyboard_disabled(&pic) {
            return Ok(FALSE);
        }
        Ok(self.would_eat(&self.key_event(vk)).into())
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let vk = wparam.0 as u32;
        self.note_key_down(vk, lparam);
        if self.keyboard_disabled(&pic) {
            return Ok(FALSE);
        }
        let event = self.key_event(vk);
        Ok(self.handle_key(pic, event).into())
    }

    fn OnTestKeyUp(&self, _pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        self.note_key_up(wparam.0 as u32);
        Ok(FALSE)
    }

    fn OnKeyUp(&self, _pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        self.note_key_up(wparam.0 as u32);
        Ok(FALSE)
    }

    /// 保留鍵命中：Ctrl+Space 直接切中英（切換鍵在 DLL 側，不進 Server）；
    /// 「翻譯選中文字」當作按下了那個組合鍵轉發給 Server（繞過 `would_eat`）。
    fn OnPreservedKey(&self, pic: Ref<ITfContext>, rguid: *const GUID) -> Result<BOOL> {
        let guid = unsafe { *rguid };
        log(&format!("保留鍵命中 guid={guid:?}"));
        if guid == preserved::GUID_SWITCH_MODE {
            if self.keyboard_disabled(&pic) {
                return Ok(FALSE);
            }
            self.set_english_mode(!self.mode_state.english());
            return Ok(true.into());
        }
        if guid != preserved::GUID_TRANSLATE || self.keyboard_disabled(&pic) {
            return Ok(FALSE);
        }
        let Some(combo) = self.translate_combo.get() else {
            return Ok(FALSE);
        };
        let event = preserved::key_event(combo, self.mode_state.english());
        Ok(self.forward_key(pic, event).into())
    }
}

impl TextService_Impl {
    /// 沒在組句時看上下文有沒有禁鍵盤（密碼框）：禁了整鍵放行、不組句。組句中不看——那段組句是我們自己的，
    /// 應用要禁會先終止它。每鍵兩次 compartment 讀取，微秒級。
    fn keyboard_disabled(&self, pic: &Ref<ITfContext>) -> bool {
        if self.shared.composing() {
            return false;
        }
        let Ok(context) = pic.ok() else {
            return false;
        };
        let disabled = crate::com::context::keyboard_disabled(context);
        if disabled {
            log("上下文停用鍵盤（密碼框），放行");
        }
        disabled
    }

    fn key_event(&self, vk: u32) -> KeyEvent {
        to_key_event(vk, self.mode_state.english())
    }

    fn note_key_down(&self, vk: u32, lparam: LPARAM) {
        self.key_tap
            .key_down(vk, lparam, self.mode_state.switch_keys());
    }

    fn note_key_up(&self, vk: u32) {
        if vk == u32::from(VK_CAPITAL.0) {
            self.mode_state.notify();
        }
        if self.key_tap.key_up(vk, self.mode_state.switch_keys()) {
            self.set_english_mode(!self.mode_state.english());
        }
    }

    /// 這個鍵吃不吃，與 Router 的分派對齊；`OnTestKeyDown` 用，無副作用。判定見 [`eats_key`]。
    ///
    /// 帶 Ctrl/Alt/Win 只有組句中的「修飾鍵 + 數字」送 Server（譯詞 / 刪候選），其餘歸應用（翻譯選中文字走保留鍵）；
    /// 字母只有「中文模式、沒在組句、按住 Shift 的大寫」歸應用（`[general] shift_letter = "compose"` 時也吃，
    /// 讓它起一段組句），其中 V / U / I 仍送 Server：雙拼下是表示式 / 問字入口；
    /// 組句中功能鍵 / 方向鍵 / 可列印字元都吃；沒在組句時數字 / 標點也先「測吃」送去轉全形（中英各有一份開關），
    /// Server 不轉的回 Passthrough 再放行；`?` 是問字字首。
    fn would_eat(&self, event: &KeyEvent) -> bool {
        let input = self.input_settings.get().unwrap_or_default();
        eats_key(
            event,
            self.shared.composing(),
            self.shared.translating(),
            input.shift_letter_compose,
            input.english_candidates,
        )
    }

    /// 不吃的鍵絕不碰組句（否則游標一移，組句會把拼音重插到別處）。
    fn handle_key(&self, pic: Ref<ITfContext>, event: KeyEvent) -> bool {
        if !self.would_eat(&event) {
            return false;
        }
        self.forward_key(pic, event)
    }

    /// 把按鍵送給 Server 並按結果更新文件；返回吃不吃。
    fn forward_key(&self, pic: Ref<ITfContext>, event: KeyEvent) -> bool {
        // 沒連上 Server（沒起、剛重啟、轉發失敗後的退避期）：只吃「可能是在打拼音」的鍵，
        // 別讓拼音字母漏進應用；標點 / 數字 / 英文與 Caps 下的字母本來就會原樣交給應用，
        // 這裡放行——一律吃掉會表現為「按了沒反應」（連不上時按 `-`、數字都沒反應）。
        if !self.ensure_connected() {
            let eat = eats_without_server(&event);
            if eat {
                log(&format!(
                    "沒連上 Server，吃掉 vk={} char={:?}",
                    event.virtual_key, event.character
                ));
            }
            return eat;
        }
        // OnTestKeyDown 已宣告吃的可列印字元，Server 放行時由輸入法自己插入：退回應用的話，企業微信 /
        // 微信 / notepad++ 這類自繪輸入框會把它丟掉。功能鍵（無字元）仍交給應用。
        let passthrough_char = event.character.filter(|c| !c.is_control());
        if let Ok(context) = pic.ok() {
            self.shared.set_last_context(Some(context.clone()));
        }
        // Server 互動在這段借用裡做完，放掉借用再走編輯會話。
        let next = {
            let mut guard = self.engine.borrow_mut();
            let Some(client) = guard.as_mut() else {
                return true;
            };
            // 組句被應用終止過：先讓 Server 清掉殘留的拼音（文本已在文件裡，交出的丟棄）。
            let response = if self.shared.take_server_stale() {
                client.commit().and_then(|_| client.key(event))
            } else {
                client.key(event)
            };
            match response {
                Ok(KeyReply::Result(response)) => {
                    // 「只在候選視窗」模式應用裡不放行內拼音（那一行由 Server 畫在候選視窗頂部）。
                    let preedit = if response.frame.preedit_mode.inline() {
                        preedit_string(&response.frame)
                    } else {
                        String::new()
                    };
                    self.shared.set_composing(!response.frame.is_empty());
                    // 翻譯評審的任何鍵都結束評審（Server 側已同步結束）。
                    self.shared.set_translating(false);
                    let consumed = matches!(response.outcome, KeyOutcome::Consumed);
                    let m = event.modifiers;
                    log(&format!(
                        "收鍵 vk={} ctrl={} alt={} shift={} caps={} en={} char={:?} candidates={} preedit={preedit:?} consumed={consumed}",
                        event.virtual_key,
                        m.ctrl,
                        m.alt,
                        m.shift,
                        m.caps,
                        m.english_mode,
                        event.character,
                        response.frame.candidates.items.len()
                    ));
                    Next::Document {
                        commit: response.commit,
                        preedit,
                        consumed,
                    }
                }
                Ok(KeyReply::NeedSelection { request }) => {
                    log(&format!("翻譯選中文字：Server 請讀選區 request={request}"));
                    Next::ReadSelection { request }
                }
                Err(error) => {
                    log(&format!("轉發按鍵失敗，放行並斷開，下一鍵重連: {error}"));
                    *guard = None;
                    self.last_connect_failure.set(None);
                    self.shared.end_composing();
                    Next::Abort
                }
            }
        };
        // 帶 Ctrl / Alt / Win 的組合（翻譯保留鍵）放行時仍交還應用，別把熱鍵的字母插進文件。
        let insertable = !event.modifiers.has_command_key();
        match (next, passthrough_char) {
            // 放行 + 沒在組句 + 可列印字元：輸入法插入，吃掉；Server 順帶交出的英文直輸段字母拼在前面。
            (
                Next::Document {
                    consumed: false,
                    commit,
                    preedit,
                },
                Some(c),
            ) if insertable && preedit.is_empty() => {
                let mut text = commit.unwrap_or_default();
                text.push(c);
                self.update_document(pic, Some(text), String::new());
                true
            }
            // 放行的功能鍵：Server 沒動緩衝區，交還應用（應用處理這個鍵時游標可能會移）。
            (
                Next::Document {
                    consumed: false, ..
                },
                _,
            ) => false,
            (
                Next::Document {
                    commit, preedit, ..
                },
                _,
            ) => {
                self.update_document(pic, commit, preedit);
                true
            }
            // 讀選區是非同步的：先吃掉這個鍵，選區文本在回撥裡發給 Server。
            (Next::ReadSelection { request }, _) => {
                self.read_selection(pic, request);
                true
            }
            (Next::Abort, _) => false,
        }
    }
}

/// 連不上 Server 時這個鍵吃不吃。
///
/// 只有「中文模式下可能是拼音」的字母要吃掉：漏進應用會變成一串字母，比什麼都不出更難看。
/// 標點 / 數字（會話外本來就走 Passthrough 交給應用）與英文模式、Caps 亮著時敲的字母都放行——
/// 一律吃掉會表現為「按了沒反應」，而且連日誌都不留（這就是「中文模式按 `-` 偶發無反應」的成因：
/// 斷連到重連成功之間的鍵全被吞了）。
fn eats_without_server(event: &KeyEvent) -> bool {
    !event.modifiers.has_command_key()
        && is_letter(event.virtual_key)
        && !(event.modifiers.caps || event.modifiers.english_mode)
}

/// 這個鍵吃不吃（[`TextService_Impl::would_eat`] 的純邏輯，便於單測）。
///
/// - 翻譯評審中一律吃，交給 Server 定接受 / 取消；
/// - 帶 Ctrl / Alt / Win：只有組句中的「修飾鍵 + 數字」吃（譯詞 / 刪候選），其餘歸應用（翻譯選中文字走保留鍵）；
/// - 沒在組句時 Caps 亮著、或英文模式而這個應用不給英文候選（`english_candidates`）：字母歸應用。
///   Server 對它們本來就只是原樣上屏，吃了反而讓應用收不到按鍵——Photoshop 畫布上的 W / H / V 快捷鍵就這樣失靈；
/// - 字母只有「中文模式、沒在組句、按住 Shift 的大寫」歸應用，其中 V / U / I 仍吃：雙拼下是表示式 / 問字入口。
///   `[general] shift_letter = "compose"`（Server 經 [`InputSettings`](qingjian_platform::protocol::InputSettings) 下發）時這種大寫也吃：送去 Core 起一段組句，
///   `⇧C` 接 `pan` 才能出「C盤」；組句一開始，後面的 Shift 字母本來就被 `composing` 兜住；
/// - 組句中功能鍵 / 方向鍵 / 可列印字元都吃；
/// - 沒在組句時數字 / 標點也先「測吃」送去轉全形（中英各有一份開關），Server 不轉的回 Passthrough 再放行；`?` 是問字字首。
fn eats_key(
    event: &KeyEvent,
    composing: bool,
    translating: bool,
    shift_letter_compose: bool,
    english_candidates: bool,
) -> bool {
    if translating {
        return true;
    }
    let modifiers = event.modifiers;
    if modifiers.has_command_key() {
        return composing && digit_key(event.virtual_key);
    }
    let vk = event.virtual_key;
    if is_letter(vk) {
        if !composing && (modifiers.caps || (modifiers.english_mode && !english_candidates)) {
            return false;
        }
        return modifiers.caps
            || modifiers.english_mode
            || !modifiers.shift
            || composing
            || shift_letter_compose
            || is_mode_letter(vk);
    }
    if composing {
        return is_edit(vk) || is_nav(vk) || event.character.is_some_and(|c| !c.is_control());
    }
    event
        .character
        .is_some_and(|c| c.is_ascii_punctuation() || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use qingjian_platform::protocol::{KeyEvent, KeyModifiers};

    use super::{eats_key, eats_without_server};
    use crate::com::key::event::to_key_event;

    /// 中文模式（`caps` / `english_mode` 都滅）。
    const CHINESE: (bool, bool) = (false, false);

    fn key(vk: u32, caps: bool, english_mode: bool) -> qingjian_platform::protocol::KeyEvent {
        let mut event = to_key_event(vk, english_mode);
        event.modifiers.caps = caps;
        event.modifiers.english_mode = english_mode;
        event
    }

    fn with_modifiers(vk: u32, character: char, modifiers: KeyModifiers) -> KeyEvent {
        let mut event = KeyEvent::new(vk, Some(character), modifiers);
        event.modifiers = modifiers;
        event
    }

    #[test]
    fn shifted_letters_without_composition_go_to_the_app() {
        // 中文模式、沒在組句：Shift 敲的大寫預設**歸應用**（原樣打出來）
        let shifted = KeyModifiers {
            shift: true,
            ..KeyModifiers::default()
        };
        assert!(!eats_key(
            &with_modifiers(0x41, 'A', shifted),
            false,
            false,
            false,
            true
        ));
        // `[general] shift_letter = "compose"`：沒在組句也吃，送去起一段組句（⇧C 接 pan 出 C盤）
        assert!(eats_key(
            &with_modifiers(0x41, 'A', shifted),
            false,
            false,
            true,
            true
        ));
        // 組句一開始，後面的 Shift 字母就被 `composing` 兜住，一律吃
        assert!(eats_key(
            &with_modifiers(0x41, 'A', shifted),
            true,
            false,
            false,
            true
        ));
        // 雙拼下 Shift + V / U / I 是表示式 / 問字入口：沒在組句也吃
        assert!(eats_key(
            &with_modifiers(0x56, 'V', shifted),
            false,
            false,
            false,
            true
        ));
        // 不帶 Shift 的字母本來就吃
        assert!(eats_key(
            &with_modifiers(0x41, 'a', KeyModifiers::default()),
            false,
            false,
            false,
            true
        ));
        // Caps 亮著（直通大寫）沒在組句：歸應用，組句中照吃（先把敲的拼音原樣上屏）
        let caps = KeyModifiers {
            caps: true,
            ..KeyModifiers::default()
        };
        assert!(!eats_key(
            &with_modifiers(0x41, 'A', caps),
            false,
            false,
            false,
            true
        ));
        assert!(eats_key(
            &with_modifiers(0x41, 'A', caps),
            true,
            false,
            false,
            true
        ));
        // 帶 Ctrl 的組合鍵歸應用，翻譯評審中一律吃
        let ctrl_c = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::default()
        };
        assert!(!eats_key(
            &with_modifiers(0x43, 'c', ctrl_c),
            false,
            false,
            false,
            true
        ));
        assert!(eats_key(
            &with_modifiers(0x43, 'c', ctrl_c),
            false,
            true,
            false,
            true
        ));
    }

    #[test]
    fn english_letters_go_to_the_app_without_english_candidates() {
        let english = KeyModifiers {
            english_mode: true,
            ..KeyModifiers::default()
        };
        let h = with_modifiers(0x48, 'h', english);
        // 不給英文候選（Photoshop 畫布上的 H 是抓手工具）：沒在組句歸應用
        assert!(!eats_key(&h, false, false, false, false));
        // 給英文候選：送去組英文詞
        assert!(eats_key(&h, false, false, false, true));
        // 組句中照吃
        assert!(eats_key(&h, true, false, false, false));
    }

    #[test]
    fn letters_are_eaten_only_in_chinese_mode() {
        let (caps, english) = CHINESE;
        assert!(eats_without_server(&key(0x41, caps, english))); // A
        assert!(!eats_without_server(&key(0x41, true, english))); // Caps 亮著是直通英文
        assert!(!eats_without_server(&key(0x41, caps, true))); // 英文模式
    }

    #[test]
    fn punctuation_digits_and_command_keys_go_to_the_app() {
        let (caps, english) = CHINESE;
        // `-`（0xBD）、數字 2、`@`：中文模式會話外都是原樣交給應用的鍵
        assert!(!eats_without_server(&key(0xBD, caps, english)));
        assert!(!eats_without_server(&key(0x32, caps, english)));
        assert!(!eats_without_server(&key(0x32, true, english)));
        // Ctrl+C 這類組合鍵一律歸應用
        let mut combo = key(0x43, caps, english);
        combo.modifiers = KeyModifiers {
            ctrl: true,
            ..combo.modifiers
        };
        assert!(!eats_without_server(&combo));
    }
}
