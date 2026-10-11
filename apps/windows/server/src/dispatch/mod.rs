//! 協議分派：把 DLL 發來的 [`ClientMessage`] 交給 Engine，產出回給 DLL 的 [`ServerMessage`]。
//! 訊息分派在 [`message`]，會話在 [`session`]，組句展示狀態在 [`composed`]，按鍵在 [`key`]，
//! 候選視窗輸出在 [`candidates`]，狀態條在 [`status`]，翻譯選中文字在 [`translate`]，配置熱載入在 [`reload`]，
//! 本地整句模型在 [`rescore`]，形碼碼錶在 [`code`]。

mod candidates;
mod code;
mod composed;
mod config;
mod key;
mod message;
mod reload;
mod rescore;
mod session;
mod status;
mod translate;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use qingjian_core::Engine;
use qingjian_platform::LocalModelConfig;
use qingjian_platform::protocol::{
    ClientMessage, Frame, IndicatorState, InputSettings, ScreenRect, ServerMessage, SessionId,
};
use qingjian_render::Mode;

pub use self::candidates::{CandidateSink, NoopSink, RenderSettings};
pub use self::code::find_code_table;
use self::composed::Composed;
pub use self::config::RouterConfig;
use self::reload::ConfigReload;
pub use self::reload::{DataDirs, attach_cloud};
pub use self::rescore::find_model;
use self::rescore::{ModelLoader, RescoreState};
use self::session::SessionInfo;
pub use self::status::{NoopStatusSink, StatusEvent, StatusSink, StatusView};
use self::translate::Translation;

/// 學習資料落盤間隔（與 macOS 殼一致）；Server 沒有定時器，借訊息節拍看時間。
const LEARNING_FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// 同一時刻只有一個應用有鍵盤焦點，所以一個 Engine 持當前組句；焦點切到別的會話時先清掉上一個的殘留。
pub struct Router {
    /// 輸入核心，程序內唯一。
    engine: Engine,

    /// 每頁候選數 / 雲端槽位 / 排布 / 外觀 / 翻頁鍵等。
    config: RouterConfig,

    /// 活躍會話及各自的宿主應用。
    sessions: HashMap<SessionId, SessionInfo>,

    /// 當前持有組句的會話。
    focused: Option<SessionId>,

    /// 當前組句的展示狀態；沒在組句時為 `None`。
    composed: Option<Composed>,

    /// 「翻譯選中文字」進行態；與 `composed` 互斥。
    translation: Option<Translation>,

    /// 已發出、等 DLL 回選區的請求號；對不上的 `Selection` 丟棄。
    pending_selection: Option<u64>,

    /// 滑鼠點候選視窗上屏、還沒交給 DLL 的文本（哪個會話的）：下一拍輪詢或下一個鍵帶走。
    clicked: Option<(SessionId, String)>,

    /// 「翻譯選中文字」請求號計數器。
    selection_seq: u64,

    /// 整句補全（preedit 右側、Tab 上屏）；緩衝變化時清空。
    sentence: Option<String>,

    /// 刪候選後的螢幕提示，隨下一幀下發、下一次按鍵清。
    notice: Option<String>,

    /// 當前高亮候選在佈局裡的下標（跨頁）。
    highlight: usize,

    /// 這輪查詢裡動過高亮：動過就不再拿重排結果換掉候選；注音模式數字鍵動過之後才選詞。
    navigated: bool,

    /// 上次把學習資料落盤的時間。
    last_flush: Instant,

    /// 配置熱載入狀態；`None` 表示不熱載入。
    reload: Option<ConfigReload>,

    /// 候選視窗輸出端；Windows 上由 [`crate::ui`] 注入。
    candidates: Box<dyn CandidateSink>,

    /// 懸浮狀態條輸出端；Windows 上由 [`crate::ui`] 注入。
    status: Box<dyn StatusSink>,

    /// 全域性中英模式（`true` 英文），所有應用共用。DLL 切了報來，啟用 / 獲焦 / 輪詢時取走。
    english: bool,

    /// 當前輸入法是不是青簡：有 DLL 來取模式就是，切成別的輸入法時收起。狀態條只在這時顯示；
    /// 應用退出不影響它，狀態條是桌面常駐的。
    ime_active: bool,

    /// 聚焦會話最近報來的游標矩形；雲聯想非同步到達時按它原地重擺候選視窗。
    last_rect: Option<ScreenRect>,

    /// 上次真正顯示的幀與位置：沒變就不重畫（組字期間的空轉 Poll 很多）。
    last_shown: Option<(Frame, Mode, ScreenRect)>,

    /// 本地整句模型（`.qjm` 或三件套目錄）；沒有模型檔案為 `None`。
    model_path: Option<PathBuf>,

    /// 形碼碼錶（`wubi/wubi86.tsv`，啟動時找好的，見 [`code::find_code_table`]）；沒有為 `None`。
    code_table: Option<PathBuf>,

    /// 進行中的模型載入；載入完接到 Engine 上就清掉。
    model_loader: Option<ModelLoader>,

    /// 上次套用的 `[model]`，變了才過載 / 解除安裝。
    applied_model: LocalModelConfig,

    /// 重排的防抖 / 輪詢進行態。
    rescore: RescoreState,
}

impl Router {
    pub fn new(engine: Engine, config: RouterConfig) -> Self {
        Self {
            engine,
            config: RouterConfig {
                page_size: config.page_size.max(1),
                ..config
            },
            sessions: HashMap::new(),
            focused: None,
            composed: None,
            translation: None,
            pending_selection: None,
            clicked: None,
            selection_seq: 0,
            sentence: None,
            notice: None,
            highlight: 0,
            navigated: false,
            last_flush: Instant::now(),
            reload: None,
            candidates: Box::new(NoopSink),
            status: Box::new(NoopStatusSink),
            english: false,
            ime_active: false,
            last_rect: None,
            last_shown: None,
            model_path: None,
            code_table: None,
            model_loader: None,
            applied_model: LocalModelConfig::default(),
            rescore: RescoreState::default(),
        }
    }

    /// 下發給 DLL 的按鍵行為設定：`OpenSession` 的回包帶一次，之後每拍 `SyncMode` 也跟著走，
    /// 所以 DLL 不用自己讀配置檔案，配置改了也不用重開會話。英文候選按這個會話的應用算。
    pub(super) fn input_settings(&self, session: SessionId) -> InputSettings {
        let app = self
            .sessions
            .get(&session)
            .and_then(|info| info.app.as_deref());
        InputSettings {
            switch_mode: self.config.switch_mode,
            english_mode: self.config.english_mode,
            shift_letter_compose: self.config.shift_letter_compose,
            english_candidates: self.config.english_candidates_in(app),
        }
    }

    /// 工作列圖示右鍵選單打勾用的開關狀態，隨 `ModeSync` 每一拍下發。
    pub(super) fn indicator_state(&self) -> IndicatorState {
        IndicatorState {
            full_width_punctuation: self.config.full_width,
            english_full_width_punctuation: self.config.english_full_width,
            status_bar: self.config.status_enabled,
            update_available: self.update_available(),
        }
    }

    pub fn set_candidate_sink(&mut self, sink: Box<dyn CandidateSink>) {
        sink.configure(self.config.render_settings());
        self.candidates = sink;
    }

    /// 直接碰 Engine：測試裡改模式鍵這類啟動時才設的開關。
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    pub fn set_status_sink(&mut self, sink: Box<dyn StatusSink>) {
        self.status = sink;
    }

    /// 處理一條訊息；`None` 表示不用回話。到點順帶把學習資料落盤。
    pub fn handle(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        let response = self.dispatch(message);
        if self.last_flush.elapsed() >= LEARNING_FLUSH_INTERVAL {
            self.flush_learning();
        }
        response
    }

    pub fn flush_learning(&mut self) {
        self.engine.flush_learning();
        self.last_flush = Instant::now();
    }
}
