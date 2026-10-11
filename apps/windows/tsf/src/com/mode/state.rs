//! 當前中 / 英模式與語言欄更新回撥，文本服務與語言欄按鈕（[`super::ModeButton`]）共享。

use core::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::UI::TextServices::{ITfLangBarItemSink, TF_LBI_ICON, TF_LBI_STATUS};

use qingjian_platform::SwitchKeys;

/// 當前中英模式 + 語言欄更新回撥，文本服務與語言欄按鈕共享（STA 單執行緒）。
pub(crate) struct ModeState {
    /// `true` 是英文模式。
    english: Cell<bool>,

    /// 內建英文模式開關（`[general] english_mode`）：關掉後誰都不許切到英文。
    enabled: Cell<bool>,

    /// 勾著的中英切換鍵（`[shortcut] switch_mode`），單擊判定與語言欄提示用。
    switch_keys: Cell<SwitchKeys>,

    /// 系統登記進來的語言欄更新回撥；由 [`super::ModeButton`] 的 `ITfSource` 登記 / 撤銷。
    pub(super) sink: RefCell<Option<ITfLangBarItemSink>>,
}

impl ModeState {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            english: Cell::new(false),
            enabled: Cell::new(true),
            switch_keys: Cell::new(SwitchKeys::default()),
            sink: RefCell::new(None),
        })
    }

    pub(crate) fn english(&self) -> bool {
        self.english.get()
    }

    pub(crate) fn set_english(&self, english: bool) {
        self.english.set(english);
    }

    /// 內建英文模式是否可用。
    pub(crate) fn enabled(&self) -> bool {
        self.enabled.get()
    }

    pub(crate) fn switch_keys(&self) -> SwitchKeys {
        self.switch_keys.get()
    }

    /// 啟用時按配置設一次。
    pub(crate) fn set_settings(&self, enabled: bool, switch_keys: SwitchKeys) {
        self.enabled.set(enabled);
        self.switch_keys.set(switch_keys);
    }

    /// 通知系統重取圖示 / 文字。
    pub(crate) fn notify(&self) {
        if let Some(sink) = self.sink.borrow().as_ref() {
            let _ = unsafe { sink.OnUpdate(TF_LBI_ICON | TF_LBI_STATUS) };
        }
    }
}
