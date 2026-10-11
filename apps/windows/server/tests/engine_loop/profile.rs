//! 「中文(台灣)」下的「青簡注音」profile：DLL 報來注音 profile，不改配置就按大千鍵盤解鍵、出繁體；按會話生效，
//! 狀態條與自繪窗跟著顯示注音。繁體 profile（「青簡」）的測試在 `composing`。

use crate::support::*;

/// 大千鍵盤敲「開發」：ㄎ=d ㄞ=9 ㄈ=z ㄚ=8，一聲是空格。
fn type_kaifa_in_dachen(router: &mut Router) -> (KeyOutcome, Option<String>, Frame) {
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    type_letters(router, "d");
    press(router, digit(9));
    press(router, space);
    type_letters(router, "z");
    press(router, digit(8));
    press(router, space)
}

fn zhuyin_profile(session: SessionId) -> ClientMessage {
    ClientMessage::Profile {
        session,
        traditional: true,
        zhuyin: true,
    }
}

#[test]
fn zhuyin_profile_decodes_dachen_keys_and_outputs_traditional() {
    let mut router = router();
    let sink = RecordingCandidates::default();
    router.set_candidate_sink(Box::new(sink.clone()));
    assert_eq!(router.handle(zhuyin_profile(SESSION)), None);

    // 配置還是全拼，但這個會話按注音解：拼音行顯示注音符號，候選是繁體
    let (_, _, frame) = type_kaifa_in_dachen(&mut router);
    assert_eq!(preedit(&frame), "ㄎㄞㄈㄚ");
    assert_eq!(candidate_texts(&frame)[0], "開發");
    let _ = router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect: rect(),
    });
    let modes = sink.1.lock().unwrap();
    let mode = modes.last().expect("自绘窗收到过帧");
    assert!(mode.traditional);
    assert_eq!(mode.scheme, "注音");
    drop(modes);

    // 注音模式下數字鍵是符號 / 聲調，選詞用 Enter
    let (_, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(commit.as_deref(), Some("開發"));
}

/// 切回同一語言下的「青簡」（拼音）：鍵盤回到配置的方案，仍出繁體。
#[test]
fn switching_from_zhuyin_profile_back_to_pinyin_restores_the_configured_scheme() {
    let mut router = router();
    let _ = router.handle(zhuyin_profile(SESSION));
    let (_, _, frame) = type_kaifa_in_dachen(&mut router);
    assert_eq!(candidate_texts(&frame)[0], "開發");
    let _ = press(&mut router, function_key(0x1B));

    let _ = router.handle(ClientMessage::Profile {
        session: SESSION,
        traditional: true,
        zhuyin: false,
    });
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    assert_eq!(preedit(&frame), "kai'fa");
    assert_eq!(candidate_texts(&frame)[0], "開發");

    // 再切回「中文(簡體)」的青簡：簡體、全拼
    let _ = press(&mut router, function_key(0x1B));
    let _ = router.handle(ClientMessage::Profile {
        session: SESSION,
        traditional: false,
        zhuyin: false,
    });
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    assert_eq!(candidate_texts(&frame)[0], "开发");
}

/// 組句中切到注音 profile：組句清掉，之後的鍵按注音解。
#[test]
fn switching_to_zhuyin_profile_mid_composition_resets_it() {
    let mut router = router();
    type_letters(&mut router, "kai");
    let _ = router.handle(zhuyin_profile(SESSION));
    let (_, _, frame) = type_letters(&mut router, "d");
    assert_eq!(preedit(&frame), "ㄎ");
}

/// 注音 profile 只管報它的那個會話：別的會話仍按配置的全拼；焦點換過去再換回來，兩邊各自正確。
#[test]
fn zhuyin_profile_is_per_session() {
    let mut router = router();
    let other = SessionId(2);
    open_session(&mut router, other, Some("notepad.exe".to_owned()));
    let _ = router.handle(zhuyin_profile(other));

    let (_, _, frame) = type_letters(&mut router, "kaifa");
    assert_eq!(candidate_texts(&frame)[0], "开发");
    let _ = press(&mut router, function_key(0x1B));

    // 焦點換到注音的會話：按大千解、出繁體
    press_in(&mut router, other, letter('d'));
    let frame = key_result(router.handle(ClientMessage::Key {
        session: other,
        event: digit(9),
    }))
    .2;
    assert_eq!(preedit(&frame), "ㄎㄞ");
    press_in(&mut router, other, function_key(0x1B));

    // 換回來：全拼、簡體
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    assert_eq!(preedit(&frame), "kai'fa");
    assert_eq!(candidate_texts(&frame)[0], "开发");
}

/// 懸浮狀態條跟聚焦會話的 profile 走：注音 profile 下顯示注音方案名。
#[test]
fn status_bar_shows_zhuyin_for_the_zhuyin_profile() {
    let config = RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    assert_eq!(recorder.calls().last(), Some(&Some("中".to_owned())));

    let _ = router.handle(zhuyin_profile(SESSION));
    assert_eq!(recorder.calls().last(), Some(&Some("中 · 注音".to_owned())));

    let _ = router.handle(ClientMessage::Profile {
        session: SESSION,
        traditional: true,
        zhuyin: false,
    });
    assert_eq!(recorder.calls().last(), Some(&Some("中".to_owned())));
}
