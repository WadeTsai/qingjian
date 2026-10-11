//! 標點全半形、表示式模式、問字模式。

use crate::support::*;

#[test]
fn chinese_punctuation_is_full_width_only_when_not_composing() {
    // 組句中標點進直輸段的老行為（`punct_commits` 關）
    let mut router = router_with(RouterConfig {
        punct_commits: false,
        ..RouterConfig::default()
    });
    // 沒在組句：逗號轉全形；數字後的點保持半形。
    let comma = KeyEvent::new(0xBC, Some(','), Default::default());
    assert_eq!(
        press(&mut router, comma),
        (
            KeyOutcome::Consumed,
            Some("，".to_owned()),
            Frame::default()
        )
    );
    press(&mut router, digit(3));
    let period = KeyEvent::new(0xBE, Some('.'), Default::default());
    assert_eq!(press(&mut router, period).0, KeyOutcome::Passthrough);
    assert_eq!(press(&mut router, period).1, Some("。".to_owned()));

    // 小鍵盤的點不跟在數字後面也保持半形。
    let keypad_period = KeyEvent::new(0x6E, Some('.'), Default::default());
    assert_eq!(press(&mut router, keypad_period).0, KeyOutcome::Passthrough);

    // 組句中：標點進英文直輸段，不轉。
    type_letters(&mut router, "ni");
    let (outcome, commit, frame) = press(&mut router, comma);
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(!frame.is_empty(), "组句应还在");

    // 狀態條上關掉全形：原樣交給應用。
    router.handle(ClientMessage::Commit { session: SESSION });
    router.handle_status_event(StatusEvent::TogglePunctuation);
    assert_eq!(press(&mut router, comma).0, KeyOutcome::Passthrough);
}

#[test]
fn expression_mode_takes_digits_and_operators() {
    let mut router = router();
    // v 開頭進表示式模式：數字不選詞、運算子進算式，Shift + 6 是 `^` 而不是刪候選鍵。
    type_letters(&mut router, "v");
    press(&mut router, digit(1));
    press(&mut router, punct('+'));
    let (outcome, commit, frame) = press(&mut router, digit(2));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "v1+2");
    assert_eq!(candidate_texts(&frame), ["3", "1+2=3"]);
    press(&mut router, digit_with(6, SHIFT));
    let (_, _, frame) = press(&mut router, digit(2));
    assert_eq!(preedit(&frame), "v1+2^2");
    assert_eq!(candidate_texts(&frame), ["5", "1+2^2=5"]);
    // 空格上屏首選並清空。
    let (outcome, commit, frame) = press(&mut router, punct(' '));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("5".to_owned()))
    );
    assert!(preedit(&frame).is_empty());
}

#[test]
fn expression_mode_spells_chinese_numerals() {
    let mut router = router();
    type_letters(&mut router, "v");
    for n in [1, 2, 3] {
        press(&mut router, digit(n));
    }
    let (_, _, frame) = press(&mut router, punct('.'));
    assert_eq!(preedit(&frame), "v123.");
    let (_, _, frame) = press(&mut router, digit(5));
    assert_eq!(
        candidate_texts(&frame),
        [
            "一百二十三点五",
            "壹佰贰拾叁点伍",
            "一百二十三元五角",
            "壹佰贰拾叁元伍角"
        ]
    );
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x1B, None, Default::default()),
    });
    type_letters(&mut router, "v");
    for n in [1, 2, 3] {
        press(&mut router, digit(n));
    }
    let (_, _, frame) = press(&mut router, punct('+'));
    // `v123+` 算不出來就沒有候選，回車上屏原文。
    assert!(candidate_texts(&frame).is_empty());
    let (_, _, frame) = press(&mut router, KeyEvent::new(0x08, None, Default::default()));
    assert_eq!(
        candidate_texts(&frame),
        [
            "一百二十三",
            "壹佰贰拾叁",
            "一百二十三元整",
            "壹佰贰拾叁元整"
        ]
    );
    press(&mut router, KeyEvent::new(0x28, None, Default::default()));
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("壹佰贰拾叁"));
}

#[test]
fn expression_mode_other_punctuation_commits_then_applies() {
    let mut router = router();
    type_letters(&mut router, "v");
    press(&mut router, digit(1));
    press(&mut router, punct('+'));
    press(&mut router, digit(2));
    // 逗號不是算式的一部分：先把首選上屏，逗號按沒在組句處理（中文模式轉全形）。
    let (outcome, commit, frame) = press(&mut router, punct(','));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("3，".to_owned()))
    );
    assert!(preedit(&frame).is_empty());
}

#[test]
fn question_key_unicode_entry_takes_digits() {
    let mut router = router();
    type_letters(&mut router, "u");
    press(&mut router, digit(4));
    type_letters(&mut router, "e");
    press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    let (_, _, frame) = press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    assert_eq!(preedit(&frame), "u4e00");
    assert_eq!(candidate_texts(&frame), ["一"]);
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("一"));
    // `u+1f600`：`+` 也進緩衝區。
    type_letters(&mut router, "u");
    press(&mut router, punct('+'));
    press(&mut router, digit(1));
    type_letters(&mut router, "f");
    press(&mut router, digit(6));
    press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    let (_, _, frame) = press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    assert_eq!(candidate_texts(&frame), ["😀"]);
}

#[test]
fn bare_question_mark_is_plain_punctuation_by_default() {
    let mut router = router();
    // 預設 `?` 不進問字：中文模式直接出全形問號，英文模式半形。
    let (outcome, commit, frame) = press(&mut router, punct('?'));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("？"));
    assert!(preedit(&frame).is_empty());
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0xBF, Some('?'), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn bare_question_mark_enters_question_mode_in_both_modes() {
    let mut router = router_asking();
    // 中文模式：`?` 進問字模式不上屏，後面的字母是問題。
    let (outcome, commit, frame) = press(&mut router, punct('?'));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "?");
    let (_, commit, frame) = type_letters(&mut router, "sangemu");
    assert_eq!(commit, None);
    assert!(preedit(&frame).starts_with('?'), "{}", preedit(&frame));
    press(&mut router, function_key(0x1B));
    // 英文模式也一樣，Caps 送來的大寫字母按小寫收。
    let (outcome, commit, frame) = press(&mut router, KeyEvent::new(0xBF, Some('?'), CAPS));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "?");
    let (_, _, frame) = press(&mut router, letter_with('S', CAPS));
    assert_eq!(preedit(&frame), "?s");
}

#[test]
fn bare_question_mark_restores_when_followed_by_other_keys() {
    let mut router = router_asking();
    // 空格只是把這個 ? 上屏（中文模式全形），不多打空格。
    press(&mut router, punct('?'));
    let (outcome, commit, frame) = press(&mut router, punct(' '));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("？".to_owned()))
    );
    assert!(preedit(&frame).is_empty());
    // 回車同樣只上屏問號，吞掉回車。
    press(&mut router, punct('?'));
    let (outcome, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("？".to_owned()))
    );
    // 其他字元：問號上屏後按沒在組句處理（逗號轉全形）。
    press(&mut router, punct('?'));
    let (_, commit, _) = press(&mut router, punct(','));
    assert_eq!(commit.as_deref(), Some("？，"));
    // 退格刪掉它，什麼都不上屏。
    press(&mut router, punct('?'));
    let (outcome, commit, frame) = press(&mut router, function_key(0x08));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(preedit(&frame).is_empty());
    // 英文模式還原成半形。
    press(&mut router, KeyEvent::new(0xBF, Some('?'), ENGLISH));
    let (_, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit.as_deref(), Some("?"));
}

#[test]
fn bare_question_mark_is_half_width_when_full_width_is_off() {
    let mut router = router_asking_with(RouterConfig {
        full_width: false,
        ..RouterConfig::default()
    });
    press(&mut router, punct('?'));
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("?"));
}

#[test]
fn shuangpin_semicolon_stays_in_buffer_in_question_mode() {
    let mut router = router_asking_with(RouterConfig {
        scheme: Scheme::Shuangpin(ShuangpinScheme::Microsoft),
        ..RouterConfig::default()
    });
    // 微軟雙拼的 `;` 是 ing 鍵：問字模式下末尾有落單聲母時進緩衝區，而不是把候選上屏。
    press(&mut router, punct('?'));
    type_letters(&mut router, "x");
    let (outcome, commit, frame) = press(&mut router, punct(';'));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(!preedit(&frame).is_empty());
    assert_ne!(preedit(&frame), "?x");
}

#[test]
fn shuangpin_enters_modes_with_shifted_letters() {
    let mut router = router_with(RouterConfig {
        scheme: Scheme::Shuangpin(ShuangpinScheme::Xiaohe),
        ..RouterConfig::default()
    });
    // Shift+V 進表示式：數字和運算子進緩衝區，空格上屏結果。
    let (outcome, commit, _) = press(&mut router, letter_with('V', SHIFT));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    press(&mut router, digit(1));
    press(&mut router, punct('+'));
    let (_, _, frame) = press(&mut router, digit(2));
    assert_eq!(preedit(&frame), "V1+2");
    assert_eq!(candidate_texts(&frame)[0], "3");
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("3"));
    // Shift+U 進問字：碼點本地答。
    press(&mut router, letter_with('U', SHIFT));
    press(&mut router, digit(4));
    type_letters(&mut router, "e");
    press(&mut router, digit(0));
    let (_, _, frame) = press(&mut router, digit(0));
    assert_eq!(candidate_texts(&frame), ["一"]);
    press(&mut router, function_key(0x1B));
    // 小寫 v 仍是音節鍵；其他大寫字母、全拼下的 Shift+V 照舊交給應用。
    let (_, _, frame) = type_letters(&mut router, "v");
    assert_eq!(preedit(&frame), "zh");
    press(&mut router, function_key(0x1B));
    assert_eq!(
        press(&mut router, letter_with('A', SHIFT)).0,
        KeyOutcome::Passthrough
    );
    let mut full = router_with(RouterConfig::default());
    let (outcome, _, frame) = press(&mut full, letter_with('V', SHIFT));
    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert!(frame.is_empty());
}

#[test]
fn punctuation_toggle_is_remembered_per_mode() {
    let mut router = router_with(RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    });
    let comma = KeyEvent::new(0xBC, Some(','), Default::default());
    let english_comma = KeyEvent::new(0xBC, Some(','), ENGLISH);
    // 中文模式下切成半形。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    router.handle_status_event(StatusEvent::TogglePunctuation);
    assert_eq!(press(&mut router, comma).0, KeyOutcome::Passthrough);
    // 英文模式預設半形；點那一格切成全形，英文模式下真轉。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
    });
    assert_eq!(press(&mut router, english_comma).0, KeyOutcome::Passthrough);
    router.handle_status_event(StatusEvent::TogglePunctuation);
    assert_eq!(press(&mut router, english_comma).1, Some("，".to_owned()));
    // 切回中文：還是中文自己記住的半形；再切回英文：還是英文記住的全形。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    assert_eq!(press(&mut router, comma).0, KeyOutcome::Passthrough);
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
    });
    assert_eq!(press(&mut router, english_comma).1, Some("，".to_owned()));
    // 英文候選組詞中敲標點：先把字母原樣上屏，標點也按英文那份轉。
    type_english(&mut router, "hello");
    let (_, commit, _) = press(&mut router, english_comma);
    assert_eq!(commit.as_deref(), Some("hello，"));
}

/// 中文模式下的 Shift 大寫：預設交給應用（與以前一致），配成 compose 才收進組句緩衝區。
#[test]
fn shift_letters_follow_the_configuration() {
    // 預設 `shift_letter = "passthrough"`：臨時打英文，字母歸應用
    let mut router = router();
    let (outcome, commit, _) = press(&mut router, letter_with('P', SHIFT));
    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert_eq!(commit, None);

    // 配成 compose：進組句、按小寫參與匹配，拼音行按敲的樣子顯示，回車原樣上屏時還原大寫
    let config = RouterConfig {
        shift_letter_compose: true,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    type_letters(&mut router, "ni");
    let (outcome, commit, frame) = press(&mut router, letter_with('A', SHIFT));
    assert_eq!((outcome, commit.as_deref()), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "niA");
    let (_, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(commit.as_deref(), Some("niA"));
}
