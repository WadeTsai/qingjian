//! 標點 / 數字按當前鍵盤佈局解析，測鍵時不改變 dead key 狀態。

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, GetKeyboardState, HKL, MAPVK_VK_TO_VSC, MapVirtualKeyExW, ToUnicodeEx,
};

pub(super) fn character(vk: u32) -> Option<char> {
    // 主鍵盤數字、小鍵盤數字與運算子（不解出字元的鍵 would_eat 會直接放行給應用）、OEM 標點和空格。
    if !matches!(vk, 0x30..=0x39 | 0x60..=0x6F | 0xBA..=0xC0 | 0xDB..=0xDE | 0x20) {
        return None;
    }
    let mut state = [0; 256];
    unsafe { GetKeyboardState(&mut state) }.ok()?;
    resolve(vk, &state, unsafe { GetKeyboardLayout(0) })
}

fn resolve(vk: u32, state: &[u8; 256], layout: HKL) -> Option<char> {
    let scan = unsafe { MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC, Some(layout)) };
    let mut buffer = [0; 8];
    // Windows 10 1607 起，bit 2 可避免測鍵修改 dead key 狀態。
    let count = unsafe { ToUnicodeEx(vk, scan, state, &mut buffer, 1 << 2, Some(layout)) };
    // 僅接受單個 UTF-16 單元；dead key、無對映和較長結果不生成字元。
    if count != 1 {
        return None;
    }
    char::from_u32(u32::from(buffer[0]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KLF_NOTELLSHELL, LoadKeyboardLayoutW, VK_NUMLOCK, VK_SHIFT,
    };
    use windows::core::w;

    #[test]
    fn punctuation_and_digits_follow_layout() {
        // 不加 KLF_ACTIVATE，不切換使用者正在使用的佈局。
        let us = unsafe { LoadKeyboardLayoutW(w!("00000409"), KLF_NOTELLSHELL) }.unwrap();
        let de = unsafe { LoadKeyboardLayoutW(w!("00000407"), KLF_NOTELLSHELL) }.unwrap();
        let mut state = [0; 256];
        assert_eq!(resolve(0xBA, &state, us), Some(';'));
        assert_eq!(resolve(0x32, &state, de), Some('2'));
        assert_eq!(resolve(0x20, &state, us), Some(' '));
        state[VK_SHIFT.0 as usize] = 0x80;
        assert_eq!(resolve(0xBA, &state, us), Some(':'));
        assert_eq!(resolve(0xBB, &state, us), Some('+'));
        assert_eq!(resolve(0xBB, &state, de), Some('*'));
        assert_eq!(resolve(0x32, &state, us), Some('@'));
        assert_eq!(resolve(0x32, &state, de), Some('"'));
    }

    /// 小鍵盤數字與運算子也要能解出字元：V 模式（表示式模式）裡敲小鍵盤才會進緩衝區。
    /// 關掉 NumLock 時系統上報的 vk 是導航鍵（VK_INSERT / VK_END…），到不了這裡。
    #[test]
    fn keypad_digits_and_operators_resolve() {
        let us = unsafe { LoadKeyboardLayoutW(w!("00000409"), KLF_NOTELLSHELL) }.unwrap();
        let mut state = [0; 256];
        state[VK_NUMLOCK.0 as usize] = 0x01;
        for (vk, expected) in [
            (0x60, '0'),
            (0x61, '1'),
            (0x62, '2'),
            (0x63, '3'),
            (0x64, '4'),
            (0x65, '5'),
            (0x66, '6'),
            (0x67, '7'),
            (0x68, '8'),
            (0x69, '9'),
            (0x6A, '*'),
            (0x6B, '+'),
            (0x6D, '-'),
            (0x6E, '.'),
            (0x6F, '/'),
        ] {
            assert_eq!(resolve(vk, &state, us), Some(expected), "vk={vk:#x}");
        }
    }

    #[test]
    fn dead_key_lookup_does_not_change_next_character() {
        let intl = unsafe { LoadKeyboardLayoutW(w!("00020409"), KLF_NOTELLSHELL) }.unwrap();
        let state = [0; 256];
        for _ in 0..2 {
            assert_eq!(resolve(0xDE, &state, intl), None);
            // 每次預讀後立即檢查，避免兩次 dead key 抵消副作用。
            assert_eq!(resolve(0x45, &state, intl), Some('e'));
        }
        assert_eq!(resolve(0, &state, intl), None);
        let mut buffer = [0; 8];
        // 與上面的重音檢查序列：核心鍵盤緩衝會讓並行測試互相影響。
        let dead = unsafe { ToUnicodeEx(0xDE, 0, &state, &mut buffer, 0, Some(intl)) };
        let result = resolve(0x31, &state, intl);
        // 清掉待組合的重音後再斷言。
        let count = unsafe { ToUnicodeEx(0x31, 0, &state, &mut buffer, 0, Some(intl)) };
        assert!(dead < 0);
        assert_eq!(count, 2);
        assert_eq!(result, None);
    }

    #[test]
    fn other_keys_are_not_resolved() {
        for vk in [0x41, 0x70, 0x0D, 0x14] {
            assert_eq!(character(vk), None);
        }
    }

    #[test]
    fn event_reads_thread_layout_and_keyboard_state() {
        use crate::com::key::event::to_key_event;
        use crate::com::service::TextService;
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            ActivateKeyboardLayout, SetKeyboardState, VK_CAPITAL, VK_CONTROL, VK_LSHIFT,
        };
        use windows::Win32::UI::TextServices::ITfKeyEventSink;
        use windows::core::ComObject;

        let original_layout = unsafe { GetKeyboardLayout(0) };
        let mut original_state = [0; 256];
        unsafe { GetKeyboardState(&mut original_state) }.unwrap();
        // 僅修改測試執行緒；即使斷言失敗，也先恢復佈局和按鍵狀態。
        let result = std::panic::catch_unwind(|| {
            let service = ComObject::new(TextService::new());
            let sink: ITfKeyEventSink = service.to_interface();
            for (name, expected) in [(w!("00000409"), '+'), (w!("00000407"), '*')] {
                let layout = unsafe { LoadKeyboardLayoutW(name, KLF_NOTELLSHELL) }.unwrap();
                unsafe { ActivateKeyboardLayout(layout, Default::default()) }.unwrap();
                let mut state = [0; 256];
                state[VK_SHIFT.0 as usize] = 0x80;
                state[VK_LSHIFT.0 as usize] = 0x80;
                unsafe { SetKeyboardState(&state) }.unwrap();
                for _ in 0..2 {
                    let event = to_key_event(0xBB, true);
                    assert_eq!(event.character, Some(expected));
                    assert!(event.modifiers.shift);
                    assert!(event.modifiers.english_mode);
                    assert!(
                        unsafe { sink.OnTestKeyDown(None, WPARAM(0xBB), LPARAM(0)) }
                            .unwrap()
                            .as_bool()
                    );
                }
                state[VK_SHIFT.0 as usize] = 0;
                state[VK_LSHIFT.0 as usize] = 0;
                state[VK_CAPITAL.0 as usize] = 1;
                state[VK_CONTROL.0 as usize] = 0x80;
                unsafe { SetKeyboardState(&state) }.unwrap();
                let event = to_key_event(0x31, false);
                assert!(!event.modifiers.shift);
                assert!(event.modifiers.caps);
                assert!(event.modifiers.ctrl);
                assert_eq!(event.virtual_key, 0x31);
                assert_eq!(to_key_event(0x41, false).character, Some('A'));
                assert!(
                    !unsafe { sink.OnTestKeyDown(None, WPARAM(0x31), LPARAM(0)) }
                        .unwrap()
                        .as_bool()
                );
            }
        });
        unsafe { ActivateKeyboardLayout(original_layout, Default::default()) }.unwrap();
        unsafe { SetKeyboardState(&original_state) }.unwrap();
        result.unwrap();
    }
}
