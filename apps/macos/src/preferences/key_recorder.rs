use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSBezelStyle, NSButton, NSEvent, NSEventModifierFlags};
use objc2_foundation::{NSObjectProtocol, NSRect, NSString};
use qingjian_platform::{KeyCombo, Modifiers};

/// 錄製中的按鈕標題。
const RECORDING_TITLE: &str = "按下新的快捷键…";

/// Esc 的鍵碼：取消錄製。
const ESCAPE_KEY: u16 = 53;

/// 錄製器的狀態。
pub struct Ivars {
    /// 只記修飾鍵（配數字鍵上屏譯詞那兩項）：按住修飾鍵再按任意鍵，鍵本身不算。
    modifiers_only: bool,

    /// 正在等使用者按鍵。
    recording: Cell<bool>,

    /// 當前值的配置寫法（`control+option+t` / `shift+option`），`changed:` 裡由 `setting_from_sender` 取走。
    recorded: RefCell<String>,

    /// 當前值給人看的寫法，取消錄製時恢復到標題上。
    label: RefCell<String>,
}

define_class!(
    // SAFETY: NSButton 允許子類化；沒有實現 Drop。
    #[unsafe(super(NSButton))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    /// 快捷鍵錄製按鈕：點一下進入錄製，按下組合鍵就記下併發 `changed:`，Esc 或失焦取消。
    /// 標題顯示當前組合（`⌃⌥T` / `⇧⌥`），配置寫法放在 ivar 裡。
    pub struct KeyRecorder;

    impl KeyRecorder {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        /// 點選：進入錄製，成為第一響應者。不調父類，免得當成普通按鈕點擊發 action。
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            if let Some(window) = self.window() {
                window.makeFirstResponder(Some(self));
            }
            self.ivars().recording.set(true);
            self.setTitle(&NSString::from_str(RECORDING_TITLE));
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            if !self.ivars().recording.get() {
                // SAFETY: 不在錄製中就按普通按鈕處理
                let _: () = unsafe { msg_send![super(self), keyDown: event] };
                return;
            }
            if event.keyCode() == ESCAPE_KEY {
                self.cancel();
                return;
            }
            let flags = event.modifierFlags();
            let modifiers = Modifiers {
                option: flags.contains(NSEventModifierFlags::Option),
                shift: flags.contains(NSEventModifierFlags::Shift),
                control: flags.contains(NSEventModifierFlags::Control),
                command: flags.contains(NSEventModifierFlags::Command),
            };
            // 沒按修飾鍵的單鍵不算快捷鍵，繼續等
            if modifiers.is_empty() {
                return;
            }
            let value = if self.ivars().modifiers_only {
                Some((modifiers.key(), modifiers.label()))
            } else {
                event
                    .charactersIgnoringModifiers()
                    .and_then(|c| c.to_string().chars().next())
                    .filter(|c| c.is_ascii_alphanumeric())
                    .map(|c| {
                        let combo = KeyCombo {
                            modifiers,
                            key: c.to_ascii_lowercase(),
                        };
                        (combo.key_string(), combo.label())
                    })
            };
            let Some((key, label)) = value else {
                return;
            };
            self.ivars().recording.set(false);
            *self.ivars().recorded.borrow_mut() = key;
            *self.ivars().label.borrow_mut() = label.clone();
            self.setTitle(&NSString::from_str(&label));
            if let Some(window) = self.window() {
                window.makeFirstResponder(None);
            }
            // target / action 由 `wire` 掛上，與其他控制元件一致
            let target: Option<Retained<AnyObject>> = self.target();
            let action = self.action();
            // SAFETY: 選擇器是 PreferencesTarget 上定義的 `changed:`，簽名 (id) -> void
            unsafe {
                self.sendAction_to(action, target.as_deref());
            }
        }

        /// 失焦：錄製沒完成就恢復原標題。
        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            if self.ivars().recording.get() {
                self.cancel();
            }
            // SAFETY: 父類的預設實現
            unsafe { msg_send![super(self), resignFirstResponder] }
        }
    }

    unsafe impl NSObjectProtocol for KeyRecorder {}
);

impl KeyRecorder {
    /// `modifiers_only` 為真時只記修飾鍵組合。
    pub fn new(mtm: MainThreadMarker, modifiers_only: bool) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(Ivars {
            modifiers_only,
            recording: Cell::new(false),
            recorded: RefCell::new(String::new()),
            label: RefCell::new(String::new()),
        });
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] };
        this.setBezelStyle(NSBezelStyle::Push);
        this
    }

    /// 按配置重新整理顯示：`key` 是配置寫法，`label` 是給人看的。
    pub fn show(&self, key: &str, label: &str) {
        if self.ivars().recording.get() {
            return;
        }
        *self.ivars().recorded.borrow_mut() = key.to_owned();
        *self.ivars().label.borrow_mut() = label.to_owned();
        self.setTitle(&NSString::from_str(label));
    }

    /// 最近一次錄到的值的配置寫法。
    pub fn recorded(&self) -> String {
        self.ivars().recorded.borrow().clone()
    }

    fn cancel(&self) {
        self.ivars().recording.set(false);
        let label = self.ivars().label.borrow().clone();
        self.setTitle(&NSString::from_str(&label));
    }
}
