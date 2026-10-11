//! 注音鍵盤排列（`[general] zhuyin_layout`）：倚天排列下按倚天的鍵解，`'` `=` 也進緩衝區；
//! 配置選注音與「青簡注音」profile 都按它。大千排列的注音流程在 `profile`。

use crate::support::*;

fn space() -> KeyEvent {
    KeyEvent::new(0x20, Some(' '), Default::default())
}

/// 倚天鍵盤敲「開發」：ㄎ=k ㄞ=i ㄈ=f ㄚ=a，一聲是空格。
fn type_kaifa_in_eten(router: &mut Router) -> (KeyOutcome, Option<String>, Frame) {
    type_letters(router, "ki");
    press(router, space());
    type_letters(router, "fa");
    press(router, space())
}

#[test]
fn eten_layout_decodes_its_own_keys_under_the_zhuyin_scheme() {
    let mut router = router_with(RouterConfig {
        scheme: Scheme::Zhuyin,
        ..RouterConfig::default()
    });
    router.engine_mut().set_zhuyin_layout(ZhuyinLayout::Eten);
    let (_, _, frame) = type_kaifa_in_eten(&mut router);
    assert_eq!(preedit(&frame), "ㄎㄞㄈㄚ");
    assert_eq!(candidate_texts(&frame)[0], "开发");
    let (_, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(commit.as_deref(), Some("开发"));
}

/// 倚天的 `'` 是 ㄘ、`=` 是 ㄦ：這兩個鍵進緩衝區，不走隔音符 / 翻頁 / 標點。
#[test]
fn eten_apostrophe_and_equals_are_zhuyin_keys() {
    let mut router = router_with(RouterConfig {
        scheme: Scheme::Zhuyin,
        ..RouterConfig::default()
    });
    router.engine_mut().set_zhuyin_layout(ZhuyinLayout::Eten);
    let _ = press(&mut router, punct('\''));
    let (_, _, frame) = press(&mut router, punct('='));
    assert_eq!(preedit(&frame), "ㄘㄦ");
}

/// 「青簡注音」profile 也按配置的排列解鍵。
#[test]
fn zhuyin_profile_follows_the_configured_layout() {
    let mut router = router();
    router.engine_mut().set_zhuyin_layout(ZhuyinLayout::Eten);
    let _ = router.handle(ClientMessage::Profile {
        session: SESSION,
        traditional: true,
        zhuyin: true,
    });
    let (_, _, frame) = type_kaifa_in_eten(&mut router);
    assert_eq!(preedit(&frame), "ㄎㄞㄈㄚ");
    assert_eq!(candidate_texts(&frame)[0], "開發");
}
