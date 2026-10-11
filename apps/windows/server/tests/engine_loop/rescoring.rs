//! 本地模型重排與隱私會話。

use crate::support::*;

#[test]
fn local_model_rescoring_reorders_sentence_after_pause() {
    // k 優路徑按末詞分狀態，幾條路徑要在末詞上不同才都留下來：ni + ta → 你他 / 你她 / 你它
    let mut router = router_with_scorer("你它");
    let (_, _, frame) = type_letters(&mut router, "nita");
    // 按鍵時只按詞級模型：他 的詞頻高，首選是「你他」
    assert_eq!(candidate_texts(&frame).first(), Some(&"你他"));
    // 在等防抖，工人迴圈該在 80 ms 內醒來
    assert!(router.next_tick() <= std::time::Duration::from_millis(80));

    let frame = tick_until_first(&mut router, "你它", std::time::Duration::from_secs(3));
    assert_eq!(
        candidate_texts(&frame).first(),
        Some(&"你它"),
        "停顿后模型偏爱的整句应换到首位，实际：{:?}",
        candidate_texts(&frame)
    );
    // 換完不再等；空閒節拍回到看配置檔案的一秒
    assert_eq!(router.next_tick(), std::time::Duration::from_secs(1));
}

#[test]
fn local_model_does_not_touch_a_navigated_page() {
    let mut router = router_with_scorer("你它");
    type_letters(&mut router, "nita");
    // 使用者動過高亮：模型的結果只留在快取裡，不換正在看的這頁
    let (_, _, frame) = press(&mut router, KeyEvent::new(0x28, None, Default::default())); // VK_DOWN
    assert_eq!(candidate_texts(&frame).first(), Some(&"你他"));
    // 防抖 80 ms + 假模型立即回分，300 ms 足夠等到結果；首選仍是原來的
    let frame = tick_until_first(&mut router, "你它", std::time::Duration::from_millis(300));
    assert_eq!(candidate_texts(&frame).first(), Some(&"你他"));
}

#[test]
fn surrounding_text_arriving_after_the_first_key_still_rescoring() {
    let mut router = router_with_scorer("你它");
    type_letters(&mut router, "nita");
    // DLL 在起組句的編輯會話裡讀到前文、按鍵之後才送來：前文換了，快取按舊前文記的作廢，要能重新排期
    assert_eq!(
        router.handle(ClientMessage::Surrounding {
            session: SESSION,
            text: "今天".to_owned(),
        }),
        None
    );
    assert!(router.next_tick() <= std::time::Duration::from_millis(80));
    let frame = tick_until_first(&mut router, "你它", std::time::Duration::from_secs(3));
    assert_eq!(candidate_texts(&frame).first(), Some(&"你它"));
    // 別的會話送來的前文不影響聚焦會話
    assert_eq!(
        router.handle(ClientMessage::Surrounding {
            session: SessionId(9),
            text: "无关".to_owned(),
        }),
        None
    );
}

/// DLL 報來「私密輸入框」：Engine 進私密（不學不記不發雲端），焦點換到別的會話按那個會話的狀態重設，切回來再進。
#[test]
fn privacy_follows_the_focused_session() {
    let mut router = router();
    // 真實順序：第一鍵起組句，DLL 在那次編輯會話裡判出私密再報來
    let (outcome, commit, _) = type_letters(&mut router, "kaifa");
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit, None);
    assert!(!router.is_private());
    assert_eq!(
        router.handle(ClientMessage::Privacy {
            session: SESSION,
            private: true,
        }),
        None
    );
    assert!(router.is_private());
    // 私密中照常上屏
    let (_, commit, _) = press(&mut router, digit(1));
    assert!(commit.is_some());
    // 另一個會話開進來拿焦點：它不私密
    open_session(&mut router, SessionId(2), None);
    press_in(&mut router, SessionId(2), letter('k'));
    assert!(!router.is_private());
    // 焦點回到第一個會話：仍是私密
    press_in(&mut router, SESSION, letter('k'));
    assert!(router.is_private());
    // 報不私密了
    assert_eq!(
        router.handle(ClientMessage::Privacy {
            session: SESSION,
            private: false,
        }),
        None
    );
    assert!(!router.is_private());
    // 別的會話的私密狀態不影響聚焦會話
    assert_eq!(
        router.handle(ClientMessage::Privacy {
            session: SessionId(9),
            private: true,
        }),
        None
    );
    assert!(!router.is_private());
}
