//! 各模組共用的零件：造 Router、造按鍵、拆回話、假的狀態條與打分器。

pub use std::path::PathBuf;
pub use std::sync::{Arc, Mutex};

pub use qingjian_core::sentence::SentenceScorer;
pub use qingjian_core::{Language, ModeKeys, ShuangpinScheme};
pub use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, ScreenRect,
    ServerMessage, SessionId,
};
pub use qingjian_platform::{
    AppsConfig, DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS, PreeditMode, Scheme,
};
pub use qingjian_render::Mode;
pub use qingjian_windows_server::dispatch::{
    CandidateSink, RenderSettings, StatusEvent, StatusSink, StatusView,
};
pub use qingjian_windows_server::{AssemblySpec, Router, RouterConfig, assembly};

pub const SESSION: SessionId = SessionId(1);

/// Caps Lock 亮著。
pub const CAPS: KeyModifiers = KeyModifiers {
    ctrl: false,
    shift: false,
    alt: false,
    win: false,
    caps: true,
    english_mode: false,
};

/// 持久英文模式（Caps 滅）。
pub const ENGLISH: KeyModifiers = KeyModifiers {
    ctrl: false,
    shift: false,
    alt: false,
    win: false,
    caps: false,
    english_mode: true,
};

/// 樣例詞庫裝一個 Router，開好一個會話。
pub fn router() -> Router {
    router_with(RouterConfig::default())
}

/// 會話開在記事本里（Windows 預設白名單裡有它，英文模式給候選）。
pub fn router_with(config: RouterConfig) -> Router {
    router_in_app_with(config, "notepad.exe")
}

/// 在某個應用（宿主 exe 名）裡開會話。
pub fn router_in_app(app: &str) -> Router {
    router_in_app_with(RouterConfig::default(), app)
}

/// 白名單固定用 Windows 預設那份：`RouterConfig::default()` 的名單跟著編譯平臺走，本機跑測試時是 macOS 的。
fn router_in_app_with(config: RouterConfig, app: &str) -> Router {
    let config = RouterConfig {
        apps: AppsConfig::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS),
        ..config
    };
    router_in(config, Some(app.to_owned()))
}

/// `?` 開著當問字入口的 Router（配置 `[shortcut] question_mark`，預設關）。
pub fn router_asking() -> Router {
    router_asking_with(RouterConfig::default())
}

pub fn router_asking_with(config: RouterConfig) -> Router {
    let mut router = router_with(config);
    router.engine_mut().set_mode_keys(ModeKeys {
        question_mark: true,
        ..ModeKeys::default()
    });
    router
}

pub fn router_in(config: RouterConfig, app: Option<String>) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dict = root.join("assets/sample/dict.tsv");
    let glossary = root.join("assets/sample/glossary-en.tsv");
    let mut engine = assembly::assemble(&AssemblySpec {
        glossary: Some((Language::English, glossary)),
        english: Some(root.join("assets/sample/english.tsv")),
        ..AssemblySpec::new(dict)
    })
    .expect("assemble engine from sample data");
    // 與 main.rs 一樣，雙拼方案是啟動時直接設給 Engine 的。
    engine.set_shuangpin(config.scheme.shuangpin());
    let mut router = Router::new(engine, config);
    // 協議版本與 Server 一致：開會話時把按鍵行為設定回一次（DLL 不讀配置檔案，靠它拿切換鍵）。
    open_session(&mut router, SESSION, app);
    router
}

/// 開一個會話並吃掉 Server 回的按鍵行為設定。
pub fn open_session(router: &mut Router, session: SessionId, app: Option<String>) {
    match router.handle(ClientMessage::OpenSession {
        session,
        app,
        protocol: PROTOCOL_VERSION,
    }) {
        Some(ServerMessage::SessionOpened { .. }) => {}
        other => panic!("expected SessionOpened, got {other:?}"),
    }
}

pub fn letter(c: char) -> KeyEvent {
    letter_with(c, Default::default())
}

/// `c` 的大小寫就是 DLL 按 Shift 解析出的字元。
pub fn letter_with(c: char, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), modifiers)
}

pub fn press(router: &mut Router, event: KeyEvent) -> (KeyOutcome, Option<String>, Frame) {
    key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event,
    }))
}

/// 英文模式下敲一串字母，返回最後一次的處理結果。
pub fn type_english(router: &mut Router, text: &str) -> (KeyOutcome, Option<String>, Frame) {
    let mut last = None;
    for c in text.chars() {
        last = Some(press(router, letter_with(c, ENGLISH)));
    }
    last.expect("typed at least one letter")
}

pub fn candidate_texts(frame: &Frame) -> Vec<&str> {
    frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect()
}

pub fn digit(n: u32) -> KeyEvent {
    digit_with(n, Default::default())
}

/// 數字鍵 1–9；`character` 按 DLL 的解析：按著 Shift 是上檔字元。
pub fn digit_with(n: u32, modifiers: KeyModifiers) -> KeyEvent {
    let c = if modifiers.shift {
        b")!@#$%^&*("[n as usize] as char
    } else {
        char::from_digit(n, 10).unwrap()
    };
    KeyEvent::new(0x30 + n, Some(c), modifiers)
}

pub const SHIFT: KeyModifiers = KeyModifiers {
    shift: true,
    ..ALT_OFF
};

pub const CTRL: KeyModifiers = KeyModifiers {
    ctrl: true,
    ..ALT_OFF
};

pub const ALT_OFF: KeyModifiers = KeyModifiers {
    ctrl: false,
    shift: false,
    alt: false,
    win: false,
    caps: false,
    english_mode: false,
};

/// 兩個平臺的預設快捷鍵都沒用 Win，拿來測「沒配到的修飾鍵歸應用」。
pub const WIN: KeyModifiers = KeyModifiers {
    win: true,
    ..ALT_OFF
};

/// 平臺預設的譯詞鍵：macOS 是 Alt，Windows 是 Ctrl（Alt 被系統選單截走）。
#[cfg(not(windows))]
pub const TRANSLATE: KeyModifiers = KeyModifiers {
    alt: true,
    ..ALT_OFF
};
#[cfg(windows)]
pub const TRANSLATE: KeyModifiers = KeyModifiers {
    ctrl: true,
    ..ALT_OFF
};

pub const TRANSLATE_SECOND: KeyModifiers = KeyModifiers {
    shift: true,
    ..TRANSLATE
};

/// 當前頁裡 `text` 排第幾（1 起）。
pub fn slot_of(frame: &Frame, text: &str) -> u32 {
    let position = candidate_texts(frame)
        .iter()
        .position(|t| *t == text)
        .unwrap_or_else(|| panic!("{text} 应在当前页：{:?}", candidate_texts(frame)));
    position as u32 + 1
}

/// 帶字元的按鍵（標點等），虛擬鍵碼隨便給一個 OEM 鍵。
pub fn punct(c: char) -> KeyEvent {
    KeyEvent::new(0xBE, Some(c), Default::default())
}

pub fn key_result(message: Option<ServerMessage>) -> (KeyOutcome, Option<String>, Frame) {
    match message {
        Some(ServerMessage::KeyResult {
            outcome,
            commit,
            frame,
            ..
        }) => (outcome, commit, frame),
        other => panic!("expected KeyResult, got {other:?}"),
    }
}

/// 中文模式下敲一串字母，返回最後一次的處理結果。
pub fn type_letters(router: &mut Router, text: &str) -> (KeyOutcome, Option<String>, Frame) {
    let mut last = None;
    for c in text.chars() {
        last = Some(key_result(router.handle(ClientMessage::Key {
            session: SESSION,
            event: letter(c),
        })));
    }
    last.expect("typed at least one letter")
}

pub fn preedit(frame: &Frame) -> String {
    frame.preedit.iter().map(|s| s.text.as_str()).collect()
}

/// 記錄狀態條呼叫：`Some(模式格文字)` 是顯示、`None` 是收起。
#[derive(Clone, Default)]
pub struct RecordingStatus(pub Arc<Mutex<Vec<Option<String>>>>);

impl RecordingStatus {
    pub fn calls(&self) -> Vec<Option<String>> {
        self.0.lock().unwrap().clone()
    }
}

impl StatusSink for RecordingStatus {
    fn show_status(&self, view: StatusView) {
        let label = match (view.english, view.scheme) {
            (true, _) => "英".to_owned(),
            (false, Some(scheme)) => format!("中 · {scheme}"),
            (false, None) => "中".to_owned(),
        };
        self.0.lock().unwrap().push(Some(label));
    }

    fn hide_status(&self) {
        self.0.lock().unwrap().push(None);
    }
}

pub fn function_key(virtual_key: u32) -> KeyEvent {
    KeyEvent::new(virtual_key, None, Default::default())
}

/// 假打分器：偏愛某個文本，其餘都給低分（與 Core 的重打分測試同款）。
pub struct Prefers(pub &'static str);

impl SentenceScorer for Prefers {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }
}

/// 接了假模型的 Router：本地整句模型在殼裡是非同步接法，按鍵先按詞級出候選，停頓後 tick 才換。
pub fn router_with_scorer(preferred: &'static str) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut engine = assembly::assemble(&AssemblySpec::new(root.join("assets/sample/dict.tsv")))
        .expect("assemble engine from sample data");
    engine.set_async_sentence_scorer(Some(Box::new(Prefers(preferred))));
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router
}

/// 一直 tick 到首選變成 `text` 或等滿 `timeout`；返回最後一幀。
pub fn tick_until_first(router: &mut Router, text: &str, timeout: std::time::Duration) -> Frame {
    let started = std::time::Instant::now();
    loop {
        std::thread::sleep(router.next_tick().min(std::time::Duration::from_millis(20)));
        router.tick();
        let frame = match router.handle(ClientMessage::Poll { session: SESSION }) {
            Some(ServerMessage::Update { frame, .. }) => frame,
            other => panic!("expected Update, got {other:?}"),
        };
        if candidate_texts(&frame).first() == Some(&text) || started.elapsed() > timeout {
            return frame;
        }
    }
}

pub fn press_in(router: &mut Router, session: SessionId, event: KeyEvent) {
    let _ = router.handle(ClientMessage::Key { session, event });
}

/// 記錄自繪候選窗收到的幀與輸入狀態。
#[derive(Clone, Default)]
pub struct RecordingCandidates(pub Arc<Mutex<Vec<Frame>>>, pub Arc<Mutex<Vec<Mode>>>);

impl CandidateSink for RecordingCandidates {
    fn show(&self, frame: Frame, mode: Mode, _rect: ScreenRect) {
        self.0.lock().unwrap().push(frame);
        self.1.lock().unwrap().push(mode);
    }

    fn hide(&self) {}

    fn configure(&self, _settings: RenderSettings) {}
}

pub fn rect() -> ScreenRect {
    ScreenRect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 20,
    }
}
