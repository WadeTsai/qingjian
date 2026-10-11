//! 組句中「修飾鍵 + 數字」：譯詞上屏與刪候選。

use crate::support::*;

#[test]
fn alt_digit_commits_first_translation() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 預設譯詞鍵（mac ⌥ / Windows Ctrl）+ 數字：上屏那個候選的第一個譯詞，組句結束。
    let (outcome, commit, after) = press(&mut router, digit_with(slot, TRANSLATE));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hello"))
    );
    assert!(after.is_empty());
}

#[test]
fn second_translation_key_without_second_sense_is_swallowed() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 樣例釋義表裡「你好」只有一條譯文：第二個譯詞鍵（Shift+譯詞鍵）+ 數字吞掉不動，組句還在。
    let (outcome, commit, after) = press(&mut router, digit_with(slot, TRANSLATE_SECOND));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&after), "ni'hao");
}

#[test]
fn shift_digit_forgets_candidate_and_requeries() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 預設 Shift + 數字：刪候選（詞庫詞只清學習記錄），重新查一遍，組句不變。
    let (outcome, commit, after) = press(&mut router, digit_with(slot, SHIFT));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&after), "ni'hao");
    assert!(!after.candidates.items.is_empty());
}

#[test]
fn unconfigured_modifier_digit_is_not_a_selection() {
    // 刪候選改成 Ctrl+Shift：Shift+4 就是普通的 `$`；Win+1 沒配到快捷鍵，歸應用。
    let mut router = router_with(RouterConfig {
        delete_keys: KeyModifiers {
            shift: true,
            ..CTRL
        },
        // 組句中標點進直輸段的老行為（`punct_commits` 關），`$` 才會留在拼音行裡
        punct_commits: false,
        ..RouterConfig::default()
    });
    type_letters(&mut router, "nihao");
    let (outcome, commit, frame) = press(&mut router, digit_with(4, SHIFT));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(preedit(&frame).contains('$'), "{}", preedit(&frame));
    let (outcome, commit, _) = press(&mut router, digit_with(1, WIN));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn deleting_a_candidate_shows_a_notice_until_next_key() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");

    // 「你好」是詞庫詞且沒學習記錄，刪不掉，但提示照樣給出。
    let (outcome, _, after) = press(&mut router, digit_with(slot, SHIFT));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(!after.is_empty(), "删候选后仍在组句");
    let notice = after.notice.as_deref().expect("删候选后应带屏幕提示");
    assert!(notice.contains("你好"), "提示应提到候选词，实际：{notice}");

    let (_, _, next) = press(&mut router, KeyEvent::new(0x28, None, Default::default())); // VK_DOWN
    assert_eq!(next.notice, None, "提示应只活到下一次按键");
}
