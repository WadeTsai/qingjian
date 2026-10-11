//! 「翻譯選中文字」與「Ctrl + Alt + Space 切換中英」兩個快捷鍵登記成 TSF **保留鍵**（preserved key）。
//! 帶 Alt 的組合是系統鍵，不經擊鍵 sink（真機：Ctrl+Alt+T 在 `OnTestKeyDown` 裡從沒出現過）；
//! 保留鍵由 TSF 在應用之前匹配、回撥 `OnPreservedKey`，UWP 裡也一樣。翻譯組合來自
//! `[shortcut] translate_selection`，啟用時讀一次配置（AppContainer 讀不到使用者目錄時用預設 Ctrl+Alt+T）；
//! 切換鍵來自 `[shortcut] switch_mode`，那個值由 Server 經協議下發（DLL 不讀配置檔案），變了就地重登記。

use windows::Win32::UI::Input::KeyboardAndMouse::VK_SPACE;
use windows::Win32::UI::TextServices::{
    ITfKeystrokeMgr, TF_MOD_ALT, TF_MOD_CONTROL, TF_MOD_SHIFT, TF_PRESERVEDKEY,
};
use windows::core::{GUID, Result};

use qingjian_platform::protocol::{KeyEvent, KeyModifiers};
use qingjian_platform::{Config, KeyCombo};

use crate::com::log::log;

/// 本保留鍵的標識，`OnPreservedKey` 按它認。
pub(crate) const GUID_TRANSLATE: GUID = GUID::from_u128(0x5c0a7b12_3d4e_4f60_8a91_2b3c4d5e6f70);

/// Ctrl + Alt + Space 中英切換鍵的保留鍵標識。
pub(crate) const GUID_SWITCH_MODE: GUID = GUID::from_u128(0x2f6b8c51_9a34_4e7d_b2c8_5d1e0f3a7b64);

/// msctf.h 的 `TF_MOD_LWIN`（windows crate 沒匯出）。
const TF_MOD_LWIN: u32 = 0x08;

/// 讀 `%APPDATA%\Qingjian\config.toml` 裡的組合；讀不到 / 解析失敗用預設。
pub(crate) fn load_combo() -> KeyCombo {
    let Some(path) = qingjian_platform::dirs::config_path() else {
        return KeyCombo::TRANSLATE_DEFAULT;
    };
    match Config::load(&path) {
        Ok(config) => config.shortcut.translate_selection,
        Err(error) => {
            log(&format!("讀配置取翻譯快捷鍵失敗，用預設: {error}"));
            KeyCombo::TRANSLATE_DEFAULT
        }
    }
}

/// Ctrl + Alt + Space 的 `TF_PRESERVEDKEY`。不用 Ctrl + Space：中文 Windows 把它綁成系統的
/// 「輸入法/非輸入法切換」，系統先截走，保留鍵收不到。
fn switch_mode_key() -> TF_PRESERVEDKEY {
    TF_PRESERVEDKEY {
        uVKey: VK_SPACE.0 as u32,
        uModifiers: TF_MOD_CONTROL | TF_MOD_ALT,
    }
}

/// 登記 Ctrl + Alt + Space 為中英切換保留鍵（`switch_mode` 勾了它時）。
pub(crate) fn register_switch_mode(keystroke: &ITfKeystrokeMgr, tid: u32) -> Result<()> {
    let description: Vec<u16> = "切換中英文（青簡）".encode_utf16().collect();
    unsafe { keystroke.PreserveKey(tid, &GUID_SWITCH_MODE, &switch_mode_key(), &description) }
}

pub(crate) fn unregister_switch_mode(keystroke: &ITfKeystrokeMgr) {
    let _ = unsafe { keystroke.UnpreserveKey(&GUID_SWITCH_MODE, &switch_mode_key()) };
}

fn preserved_key(combo: KeyCombo) -> TF_PRESERVEDKEY {
    let m = combo.modifiers;
    let mut modifiers = 0;
    if m.control {
        modifiers |= TF_MOD_CONTROL;
    }
    if m.option {
        modifiers |= TF_MOD_ALT;
    }
    if m.shift {
        modifiers |= TF_MOD_SHIFT;
    }
    if m.command {
        modifiers |= TF_MOD_LWIN;
    }
    TF_PRESERVEDKEY {
        uVKey: combo.key.to_ascii_uppercase() as u32,
        uModifiers: modifiers,
    }
}

pub(crate) fn register(keystroke: &ITfKeystrokeMgr, tid: u32, combo: KeyCombo) -> Result<()> {
    let key = preserved_key(combo);
    let description: Vec<u16> = "翻譯選中文字".encode_utf16().collect();
    unsafe { keystroke.PreserveKey(tid, &GUID_TRANSLATE, &key, &description) }
}

pub(crate) fn unregister(keystroke: &ITfKeystrokeMgr, combo: KeyCombo) {
    let key = preserved_key(combo);
    let _ = unsafe { keystroke.UnpreserveKey(&GUID_TRANSLATE, &key) };
}

/// 保留鍵命中時餵給 Server 的按鍵：Router 按字元 + 物理修飾鍵與配置比對。
pub(crate) fn key_event(combo: KeyCombo, english_mode: bool) -> KeyEvent {
    let m = combo.modifiers;
    KeyEvent::new(
        combo.key.to_ascii_uppercase() as u32,
        Some(combo.key),
        KeyModifiers {
            ctrl: m.control,
            shift: m.shift,
            alt: m.option,
            win: m.command,
            caps: false,
            english_mode,
        },
    )
}
