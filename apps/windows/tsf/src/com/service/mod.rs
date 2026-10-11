//! 文本服務物件 [`TextService`]：每執行緒一個，實現 `ITfTextInputProcessor`（啟用 / 停用，[`processor`]）、
//! `ITfKeyEventSink`（收鍵，[`key_sink`]）與顯示屬性提供者（[`display`]）。
//! 連 Server 在 [`connection`]，中英模式在 [`mode`]，往文件寫字在 [`document`]。

mod connection;
mod display;
mod document;
mod key_sink;
mod launch;
mod menu;
mod mode;
mod next;
mod processor;
mod profile;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::TextServices::{
    ITfDisplayAttributeProvider, ITfKeyEventSink, ITfLangBarItemButton, ITfSource,
    ITfTextInputProcessor, ITfThreadMgr,
};
use windows::core::{ComObject, implement};

use qingjian_platform::KeyCombo;
use qingjian_platform::protocol::{IndicatorState, InputSettings};

use super::composition::Shared;
use super::key::KeyTap;
use super::mode::ModeState;
use super::mode::sink::Advice;
use super::poll::PollTimer;
use super::profile::ProfileFlags;
use crate::client::EngineClient;
use crate::client::pipe::PipeStream;

/// 連 Server 的會話客戶端，與編輯會話 / 輪詢定時器共享（STA 單執行緒）。連不上時為 `None`，鍵照樣放行。
pub(crate) type SharedClient = Rc<RefCell<Option<EngineClient<PipeStream>>>>;

/// 連不上 Server 後隔多久再試（每次嘗試都在應用的 UI 執行緒上，不能每鍵都試）。
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);

/// 一個 TSF 文本服務例項（每執行緒一個）。
#[implement(ITfTextInputProcessor, ITfKeyEventSink, ITfDisplayAttributeProvider)]
pub struct TextService {
    /// 啟用時拿到的執行緒管理器，停用時用它反註冊。
    thread_mgr: RefCell<Option<ITfThreadMgr>>,

    /// TSF 分配的 client id。
    client_id: Cell<u32>,

    /// 引擎層。
    engine: SharedClient,

    /// 跨按鍵存活的組句狀態。
    shared: Rc<Shared>,

    /// 雲聯想輪詢定時器；掛失敗時為 `None`，退化為只在按鍵時收雲結果。
    poll_timer: RefCell<Option<PollTimer>>,

    /// 上次連 Server 失敗的時間，按 [`RECONNECT_INTERVAL`] 退避。
    last_connect_failure: Cell<Option<Instant>>,

    /// 中 / 英模式（單擊切換鍵翻轉，見 `[shortcut] switch_mode`），與語言欄按鈕共用。
    mode_state: Rc<ModeState>,

    /// 登記在系統語言欄上的中 / 英按鈕；停用時反註冊。
    mode_button: RefCell<Option<ITfLangBarItemButton>>,

    /// 「轉換模式」與「輸入法開 / 關」兩條 compartment 的事件回撥；停用時撤掉。
    mode_sinks: RefCell<Advice>,

    /// 執行緒焦點通知（source + cookie），見 [`super::focus`]；停用時撤掉。
    focus_sink: RefCell<Option<(ITfSource, u32)>>,

    /// 單擊中英切換鍵切中英的判定。
    key_tap: KeyTap,

    /// 語言 profile 通知掛上後的 cookie；掛一次就夠（見 [`super::profile`]）。
    profile_cookie: Cell<Option<u32>>,

    /// 登記成保留鍵的「翻譯選中文字」組合；停用時撤掉（見 [`preserved`](crate::com::key::preserved)）。
    translate_combo: Cell<Option<KeyCombo>>,

    /// Ctrl + Alt + Space 切換鍵當前是否已登記為保留鍵（`[shortcut] switch_mode` 勾了它時才有）。
    switch_preserved: Cell<bool>,

    /// 上一次應用過的按鍵行為設定；與 Server 下發的一致時就不重複應用
    /// （每一拍 `SyncMode` 都帶著它，見 [`TextService_Impl::apply_input_settings`]）。
    input_settings: Cell<Option<InputSettings>>,

    /// 右鍵選單打勾用的開關狀態，Server 隨 `SyncMode` 每一拍帶下來。
    indicator_state: Cell<IndicatorState>,

    /// 啟用後一小段時間內忽略轉換模式 compartment 的變化，見 [`TextService_Impl::sync_from_conversion_mode`]。
    conversion_guard_until: Cell<Option<Instant>>,

    /// 上次報給 Server 的本執行緒 profile 的輸入行為（簡 / 繁、注音與否）；`None` 是這條連線上還沒報過（見 [`profile`]）。
    profile_flags: Cell<Option<ProfileFlags>>,
}

thread_local! {
    /// 本執行緒當前啟用的文本服務，供 compartment 回撥 / 輪詢定時器切模式。`Activate` 設、`Deactivate` 清。
    static ACTIVE: RefCell<Option<ComObject<TextService>>> = const { RefCell::new(None) };
}

fn with_active(f: impl FnOnce(&TextService_Impl)) {
    let service = ACTIVE.with(|active| active.borrow().clone());
    if let Some(service) = service {
        f(&service);
    }
}

/// 使用者點了語言欄的中 / 英按鈕（見 [`ModeButton`](crate::com::mode::ModeButton)）：翻轉模式。
pub(super) fn toggle_mode() {
    with_active(|service| service.set_english_mode(!service.mode_state.english()));
}

/// 右鍵點了語言欄的中 / 英按鈕：在 `point`（螢幕座標）彈選單。
pub(super) fn show_indicator_menu(point: POINT) {
    with_active(|service| service.show_indicator_menu(point));
}

/// 「轉換模式」compartment 變了（見 [`sink`](crate::com::mode::sink)）。
pub(super) fn on_conversion_mode_changed() {
    with_active(TextService_Impl::sync_from_conversion_mode);
}

/// 「輸入法開 / 關」compartment 變了（見 [`sink`](crate::com::mode::sink)）：系統的 Ctrl + Space。
pub(super) fn on_keyboard_open_changed() {
    with_active(TextService_Impl::sync_from_keyboard_open);
}

/// 本執行緒得到 / 失去了鍵盤焦點（見 [`super::focus`]）。
pub(super) fn on_thread_focus(foreground: bool) {
    with_active(|service| service.set_thread_focus(foreground));
}

/// 輪詢發現前臺會話沒連著 Server（見 [`super::poll`]）：補連一次。
pub(super) fn on_reconnect_tick() {
    with_active(|service| {
        service.ensure_connected();
    });
}

/// 輪詢取到了 Server 的全域性模式（見 [`super::poll`]）；與當前相同就不動。
pub(super) fn on_mode_sync(english: bool) {
    with_active(|service| service.adopt_mode(english));
}

/// 輪詢取回了 Server 下發的按鍵行為設定（見 [`super::poll`]）：切換鍵 / 內建英文模式改了就地應用。
pub(super) fn on_input_settings(input: InputSettings) {
    with_active(|service| service.apply_input_settings(input));
}

/// 輪詢取回了右鍵選單打勾用的開關狀態。
pub(super) fn on_indicator_state(state: IndicatorState) {
    with_active(|service| service.indicator_state.set(state));
}

/// 本執行緒切到了青簡的某個語言 profile（見 [`crate::com::profile`]）：把它的輸入行為（簡 / 繁、注音與否）報給 Server。
/// 啟用過程中來的通知到不了這裡（`ACTIVE` 還沒設），那一次由 `connect` 自己查。
pub(super) fn on_profile_activated(flags: ProfileFlags) {
    with_active(|service| service.adopt_profile(flags));
}

impl TextService {
    #[allow(clippy::new_without_default)] // 有 lock_module 副作用
    pub fn new() -> Self {
        crate::com::lock_module();
        let engine: SharedClient = Rc::new(RefCell::new(None));
        Self {
            thread_mgr: RefCell::new(None),
            client_id: Cell::new(0),
            shared: Shared::new(engine.clone()),
            engine,
            poll_timer: RefCell::new(None),
            last_connect_failure: Cell::new(None),
            mode_state: ModeState::new(),
            mode_button: RefCell::new(None),
            mode_sinks: RefCell::new(Vec::new()),
            focus_sink: RefCell::new(None),
            key_tap: KeyTap::default(),
            profile_cookie: Cell::new(None),
            translate_combo: Cell::new(None),
            switch_preserved: Cell::new(false),
            input_settings: Cell::new(None),
            indicator_state: Cell::new(IndicatorState::default()),
            conversion_guard_until: Cell::new(None),
            profile_flags: Cell::new(None),
        }
    }
}

impl Drop for TextService {
    fn drop(&mut self) {
        crate::com::unlock_module();
    }
}
