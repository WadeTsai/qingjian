//! IMK 輸入控制器：每個輸入會話（每個應用的文本框）一個例項。
//!
//! 只做兩件事：把按鍵翻譯成 Engine 的呼叫，把 Engine 返回的候選交給候選視窗。
//! **這裡不允許出現排序、詞庫或翻譯邏輯。** 會話狀態（候選、高亮、頁碼）在 [`crate::host::Session`]。

use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, msg_send, sel};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSMenu};
use objc2_foundation::NSObjectProtocol;
use objc2_input_method_kit::{IMKInputController, IMKServer};
use qingjian_core::{Candidate, QUESTION_PREFIX};
use qingjian_platform::Modifiers;

use super::{TextClient, catch_panic, modifiers, recover_from_panic, secure_input};
use crate::candidates::Preedit;
use crate::host;
use crate::menubar;

mod click;
mod command;
mod commit;
mod display;
mod text;
mod translate;

pub use click::click;

define_class!(
    // SAFETY:
    // - IMKInputController 允許子類化，Apple 文件的標準用法就是繼承它。
    // - 沒有實現 Drop。
    #[unsafe(super(IMKInputController))]
    // 名字要和 Info.plist 的 InputMethodServerControllerClass 一致
    #[name = "QingjianInputController"]
    #[ivars = ()]
    pub struct QingjianInputController;

    impl QingjianInputController {
        /// IMKServer 為每個新會話呼叫的指定初始化方法，在這裡放好 ivars。
        #[unsafe(method_id(initWithServer:delegate:client:))]
        fn init_with_server(
            this: Allocated<Self>,
            server: Option<&IMKServer>,
            delegate: Option<&AnyObject>,
            client: Option<&AnyObject>,
        ) -> Option<Retained<Self>> {
            tracing::info!("新建输入会话");
            let this = this.set_ivars(());
            unsafe { msg_send![super(this), initWithServer: server, delegate: delegate, client: client] }
        }

        /// 所有按鍵事件都到這裡（IMK 第一層協議）。IMK 按控制器實現了哪一層決定路線，實現了這個方法就不會再分發成
        /// `inputText:client:` / `didCommandBySelector:client:`（父類預設實現也不分發），所以自己分：
        /// Option+數字上屏候選的譯文（Option 會把數字鍵變成 ¡™£ 這類字元，只能按鍵碼認）；命令鍵按鍵碼對映成原來的選擇器；
        /// 其餘按事件帶的字元走文本路徑。返回 true 表示已處理，系統不再把按鍵交給應用。
        // define_class! 會把返回型別轉成 ObjC BOOL，方法體裡不能用 `return`，邏輯放在下面的 inherent impl
        #[unsafe(method(handleEvent:client:))]
        fn handle_event(&self, event: Option<&NSEvent>, client: Option<&AnyObject>) -> bool {
            match (event, client) {
                (Some(event), Some(client)) => {
                    let client = TextClient::new(client);
                    // panic 攔下後把緩衝區原樣上屏，這個按鍵交還給應用
                    catch_panic("handleEvent", || self.dispatch_event(event, client))
                        .unwrap_or_else(|| {
                            recover_from_panic(Some(client));
                            false
                        })
                }
                _ => false,
            }
        }

        /// 應用要求立刻結束本次輸入（切換焦點、切換輸入法等）。
        #[unsafe(method(commitComposition:))]
        fn commit_composition(&self, client: Option<&AnyObject>) {
            let client = client.map(TextClient::new);
            let done = catch_panic("commitComposition", || {
                if let Some(client) = client {
                    self.commit_raw(client);
                }
                host::with(|h| {
                    h.cancel_prediction();
                    h.window.hide();
                });
            });
            if done.is_none() {
                recover_from_panic(client);
            }
        }

        #[unsafe(method(activateServer:))]
        fn activate_server(&self, sender: Option<&AnyObject>) {
            tracing::info!("activateServer");
            self.set_active();
            let done = catch_panic("activateServer", || {
                // 使用者要往 [apps] 里加應用時，從這條日誌抄 bundle identifier
                let bundle = sender.and_then(|s| TextClient::new(s).bundle_identifier());
                if let Some(bundle) = &bundle {
                    tracing::debug!(%bundle, "当前应用");
                }
                host::with(|h| {
                    h.engine.set_application(bundle);
                    h.refresh_text_replacements();
                    h.reload_config_if_changed();
                    h.indicator.activate();
                    h.watch.start();
                });
            });
            if done.is_none() {
                recover_from_panic(None);
            }
        }

        /// 系統輸入源選單（選單欄旗幟圖示）每次展開前來取輸入法自己的條目。
        #[unsafe(method_id(menu))]
        fn menu(&self) -> Option<Retained<NSMenu>> {
            host::with(|h| h.menu.ns_menu())
        }

        /// 輸入源選單裡點了條目：IMK 轉發到控制器，sender 是帶 IMKCommandMenuItem 的字典。
        #[unsafe(method(menuAction:))]
        fn menu_action(&self, sender: Option<&AnyObject>) {
            if let Some(action) = menubar::action_from_sender(sender) {
                host::with(|h| h.perform(action));
            }
        }

        #[unsafe(method(deactivateServer:))]
        fn deactivate_server(&self, sender: Option<&AnyObject>) {
            tracing::info!("deactivateServer");
            self.clear_active();
            let client = sender.map(TextClient::new);
            let done = catch_panic("deactivateServer", || {
                if let Some(client) = client {
                    self.commit_raw(client);
                }
                // 切換輸入源時無論如何都收掉候選框，不能留一個孤兒視窗在螢幕上
                host::with(|h| {
                    h.cancel_prediction();
                    h.window.hide();
                    h.indicator.deactivate();
                    h.watch.stop();
                    h.engine.break_chain();
                    h.engine.flush_learning();
                    h.last_flush = std::time::Instant::now();
                });
            });
            if done.is_none() {
                recover_from_panic(client);
                // 善後裡沒做的收尾：學習資料還是要落盤
                host::with(|h| {
                    h.indicator.deactivate();
                    h.watch.stop();
                    h.engine.flush_learning();
                });
            }
        }
    }

    unsafe impl NSObjectProtocol for QingjianInputController {}
);

/// 翻譯選中文字最多接受多少個字元：再長既慢又貴，也不是輸入法該乾的事。
const MAX_TRANSLATE_CHARS: usize = 500;

/// 給本地整句模型看的游標前文最多讀多少字元（Engine 自己再按它的前文長度截）。
const RESCORE_LOOKBACK: usize = qingjian_core::RESCORE_CONTEXT_CHARS;

/// 登入 / 鎖屏視窗的 bundle identifier。
const LOGIN_WINDOW: &str = "com.apple.loginwindow";

/// 數字行與小鍵盤的鍵碼對應的數字 1–9（ANSI 佈局的物理鍵）。
fn digit_key(key_code: u16) -> Option<usize> {
    Some(match key_code {
        18 | 83 => 1,
        19 | 84 => 2,
        20 | 85 => 3,
        21 | 86 => 4,
        23 | 87 => 5,
        22 | 88 => 6,
        26 | 89 => 7,
        28 | 91 => 8,
        25 | 92 => 9,
        _ => return None,
    })
}

impl QingjianInputController {
    /// 登入 / 鎖屏視窗：輸入源選單裡沒有青簡，loginwindow 卻照樣啟用它，按鍵一律交還系統。
    ///
    /// TODO(#190): 臨時防護。現象是開機登入介面打不進模式鍵（u / i），推斷為按鍵進了青簡的組句；
    /// 日誌只證實 loginwindow 會啟用青簡，按鍵是否真的送來沒有復現（開 FileVault 的機器進不到這個介面）。
    /// 找到按鍵送進來的條件後改成針對它的判斷，並確認別的系統介面有沒有同樣的情況。
    fn in_login_window(&self) -> bool {
        host::with(|h| h.engine.application() == Some(LOGIN_WINDOW)).unwrap_or(false)
    }

    /// 一個按鍵事件的分發：只管按下；Cmd / Ctrl 組合除 Cmd+左右外一律交給應用；命令鍵對映成選擇器；其餘按字元當文本。
    fn dispatch_event(&self, event: &NSEvent, client: TextClient<'_>) -> bool {
        if event.r#type() != NSEventType::KeyDown || self.in_login_window() {
            return false;
        }
        let flags = event.modifierFlags();
        let (command, control, option, shift) = (
            flags.contains(NSEventModifierFlags::Command),
            flags.contains(NSEventModifierFlags::Control),
            flags.contains(NSEventModifierFlags::Option),
            flags.contains(NSEventModifierFlags::Shift),
        );
        let key = event.keyCode();
        let pressed = Modifiers {
            option,
            shift,
            control,
            command,
        };
        // 提示在顯示：敲任何鍵先收掉，鍵照常處理
        host::with(|h| h.clear_notice());
        // 翻譯選中文字進行中：回車 / 空格 / 1 接受，Esc 放棄，其他鍵放棄後照常交給應用
        if host::with(|h| h.translation.is_some()).unwrap_or(false) {
            return self.handle_translation_review(key, client);
        }
        // 翻譯快捷鍵（不在組句中）：讀應用裡的選區，交給雲端
        let typed = event
            .charactersIgnoringModifiers()
            .map(|c| c.to_string().to_ascii_lowercase());
        let combo = host::with(|h| h.translate_keys).unwrap_or_default();
        if pressed == combo.modifiers
            && typed.as_deref().and_then(|t| t.chars().next()) == Some(combo.key)
            && !host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false)
        {
            return self.translate_selection(client);
        }
        // 修飾鍵 + 數字：按配置的兩組組合上屏第一 / 第二個譯詞（預設 ⌥ 與 ⇧⌥）、刪候選（預設 ⇧）。
        // 只在組句中認：不在組句時 ⇧4 就是 `$`，得走下面的標點轉換（中文模式出 ￥、⇧6 出 ……、⇧1 出 ！），
        // 以前在這裡被截走後原樣還給應用，全形轉換就沒機會做了。
        // 表示式模式（`v2^3`）裡 ⇧+數字打的是 `^ * ( )`，不當快捷鍵
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        let expression = composing && host::with(|h| h.engine.expression_mode()).unwrap_or(false);
        if composing
            && !expression
            && !pressed.is_empty()
            && let Some(digit) = digit_key(key)
        {
            let (first, second) = host::with(|h| h.translation_keys).unwrap_or_default();
            if pressed == first {
                return self.handle_translation_key(digit, 0, client);
            }
            if pressed == second {
                return self.handle_translation_key(digit, 1, client);
            }
            if pressed == host::with(|h| h.delete_keys).unwrap_or_default() {
                return self.handle_delete_key(digit, client);
            }
        }
        let selector = match key {
            36 | 76 => Some(sel!(insertNewline:)),
            48 if shift => Some(sel!(insertBacktab:)),
            48 => Some(sel!(insertTab:)),
            51 if option => Some(sel!(deleteWordBackward:)),
            51 if command => Some(sel!(deleteToBeginningOfLine:)),
            51 => Some(sel!(deleteBackward:)),
            117 => Some(sel!(deleteForward:)),
            53 => Some(sel!(cancelOperation:)),
            126 => Some(sel!(moveUp:)),
            125 => Some(sel!(moveDown:)),
            123 if command => Some(sel!(moveToLeftEndOfLine:)),
            124 if command => Some(sel!(moveToRightEndOfLine:)),
            123 if option => Some(sel!(moveWordLeft:)),
            124 if option => Some(sel!(moveWordRight:)),
            123 => Some(sel!(moveLeft:)),
            124 => Some(sel!(moveRight:)),
            116 => Some(sel!(pageUp:)),
            121 => Some(sel!(pageDown:)),
            115 => Some(sel!(moveToBeginningOfLine:)),
            119 => Some(sel!(moveToEndOfLine:)),
            _ => None,
        };
        if let Some(selector) = selector {
            return self.handle_command(selector, client);
        }
        if command || control {
            return false;
        }
        match event.characters() {
            Some(text) if !text.is_empty() => self.handle_text(&text.to_string(), client),
            _ => false,
        }
    }
}
