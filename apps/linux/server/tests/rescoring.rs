//! 本地整句模型：停頓後重排換首選、動過高亮不換、模型在組句中途接上也補這一輪；
//! 外掛定時 Poll 回的幀內容沒變時展示身份不變，重排換了順序才推進。
use std::time::{Duration, Instant};

use qingjian_core::Engine;
use qingjian_core::sentence::SentenceScorer;
use qingjian_dictionary::Dictionary;
use qingjian_linux_server::{Router, RouterConfig};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use serde_json::{Value, json};

const SESSION: SessionId = SessionId(1);

/// 假打分器：偏愛某個文本，其餘都給低分（與 Core / Windows Server 的重打分測試同款）。
struct Prefers(&'static str);

impl SentenceScorer for Prefers {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }
}

/// k 優路徑按末詞分狀態，幾條路徑要在末詞上不同才都留下來：ni + ta → 你他 / 你她 / 你它。
fn engine() -> Engine {
    Engine::new(Dictionary::parse("你\tni\t100\n他\tta\t100\n她\tta\t90\n它\tta\t80\n").unwrap())
}

fn router_with(engine: Engine) -> Router {
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: false,
    });
    router
}

/// 接了假模型的 Router：本地整句模型在 Server 裡是非同步接法，按鍵先按詞級出候選，停頓後 tick 才換。
fn router_with_scorer(preferred: &'static str) -> Router {
    let mut engine = engine();
    engine.set_async_sentence_scorer(Some(Box::new(Prefers(preferred))));
    router_with(engine)
}

fn press(router: &mut Router, event: KeyEvent) -> Frame {
    match router.handle(ClientMessage::Key {
        session: SESSION,
        event,
    }) {
        Some(ServerMessage::KeyResult { frame, .. }) => frame,
        other => panic!("expected KeyResult, got {other:?}"),
    }
}

fn type_letters(router: &mut Router, text: &str) -> Frame {
    let mut last = Frame::default();
    for c in text.chars() {
        last = press(
            router,
            KeyEvent::new(
                c.to_ascii_uppercase() as u32,
                Some(c),
                KeyModifiers::default(),
            ),
        );
    }
    last
}

fn first(frame: &Frame) -> Option<&str> {
    frame.candidates.items.first().map(|c| c.text.as_str())
}

fn poll(router: &mut Router) -> Frame {
    match router.handle(ClientMessage::Poll { session: SESSION }) {
        Some(ServerMessage::Update { frame, .. }) => frame,
        other => panic!("expected Update, got {other:?}"),
    }
}

/// 一直 tick 到首選變成 `text` 或等滿 `timeout`；返回最後一幀。
fn tick_until_first(router: &mut Router, text: &str, timeout: Duration) -> Frame {
    let started = Instant::now();
    loop {
        std::thread::sleep(router.next_tick().min(Duration::from_millis(20)));
        router.tick();
        let frame = poll(router);
        if first(&frame) == Some(text) || started.elapsed() > timeout {
            return frame;
        }
    }
}

#[test]
fn local_model_rescoring_reorders_sentence_after_pause() {
    let mut router = router_with_scorer("你它");
    let frame = type_letters(&mut router, "nita");
    // 按鍵時只按詞級模型：他 的詞頻高，首選是「你他」
    assert_eq!(first(&frame), Some("你他"));
    // 在等防抖，主迴圈該在 80 ms 內醒來
    assert!(router.next_tick() <= Duration::from_millis(80));

    let frame = tick_until_first(&mut router, "你它", Duration::from_secs(3));
    assert_eq!(
        first(&frame),
        Some("你它"),
        "停顿后模型偏爱的整句应换到首位，实际：{:?}",
        frame.candidates.items
    );
    // 換完不再等；空閒節拍回到落盤學習的一秒
    assert_eq!(router.next_tick(), Duration::from_secs(1));
}

#[test]
fn local_model_does_not_touch_a_navigated_page() {
    let mut router = router_with_scorer("你它");
    type_letters(&mut router, "nita");
    // 使用者動過高亮（下方向鍵）：模型的結果只留在快取裡，不換正在看的這頁
    let frame = press(
        &mut router,
        KeyEvent::new(0x28, None, KeyModifiers::default()),
    );
    assert_eq!(first(&frame), Some("你他"));
    // 防抖 80 ms + 假模型立即回分，300 ms 足夠等到結果；首選仍是原來的
    let frame = tick_until_first(&mut router, "你它", Duration::from_millis(300));
    assert_eq!(first(&frame), Some("你他"));
}

#[test]
fn model_attached_during_composition_rescores_the_current_round() {
    // 模型還在後臺載入時使用者已經在組句：這一輪的查詢從沒見過打分器
    let mut router = router_with(engine());
    let frame = type_letters(&mut router, "nita");
    assert_eq!(first(&frame), Some("你他"));
    assert_eq!(router.next_tick(), Duration::from_secs(1));

    // 載入完成接上：不用再敲鍵，這一輪也補一次重排
    router.attach_sentence_scorer(Box::new(Prefers("你它")));
    assert!(router.next_tick() <= Duration::from_millis(80));
    let frame = tick_until_first(&mut router, "你它", Duration::from_secs(3));
    assert_eq!(first(&frame), Some("你它"));
}

/// 外掛走 JSON 通道：握手報了展示身份，之後每幀都帶 `identity`。
fn poll_json(router: &mut Router) -> Value {
    router
        .handle_linux(json!({"Poll": {"session": 1}}))
        .expect("poll answers")
}

#[test]
fn poll_keeps_the_display_identity_until_the_frame_changes() {
    let mut router = router_with(engine());
    router.handle_linux(json!({"DisplayReporting": {"session": 1, "identity": {"generation": 2, "context": "first", "revision": 0}}}));
    let mut shown = Value::Null;
    for c in "nita".chars() {
        let key = ClientMessage::Key {
            session: SESSION,
            event: KeyEvent::new(
                c.to_ascii_uppercase() as u32,
                Some(c),
                KeyModifiers::default(),
            ),
        };
        shown = router
            .handle_linux(serde_json::to_value(key).unwrap())
            .unwrap()["KeyResult"]["identity"]
            .take();
    }
    // 組句期間外掛每 80 ms 問一次：內容沒變就沿用身份，外掛不重畫、不重複回報曝光
    for _ in 0..3 {
        let update = poll_json(&mut router);
        assert_eq!(update["Update"]["identity"], shown);
        assert_eq!(
            update["Update"]["frame"]["candidates"]["items"][0]["text"],
            "你他"
        );
    }

    // 模型接上、重排換了順序：這一幀才是新的展示，版本號推進
    router.attach_sentence_scorer(Box::new(Prefers("你它")));
    let started = Instant::now();
    let changed = loop {
        std::thread::sleep(Duration::from_millis(20));
        router.tick();
        let update = poll_json(&mut router);
        if update["Update"]["frame"]["candidates"]["items"][0]["text"] == "你它"
            || started.elapsed() > Duration::from_secs(3)
        {
            break update;
        }
    };
    assert_eq!(
        changed["Update"]["frame"]["candidates"]["items"][0]["text"],
        "你它"
    );
    assert!(changed["Update"]["identity"]["revision"].as_u64() > shown["revision"].as_u64());
    // 換完之後再問：又是沿用
    assert_eq!(
        poll_json(&mut router)["Update"]["identity"],
        changed["Update"]["identity"]
    );
}
