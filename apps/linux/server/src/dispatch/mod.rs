//! Linux 會話路由：獨立儲存各上下文的組句，詞庫和學習服務保持單例項。本地整句模型在 [`rescore`]。

mod composed;
mod config;
mod display;
mod key;
mod linux;
mod message;
mod rescore;
mod session;

use self::composed::Composed;
pub use self::config::RouterConfig;
pub use self::rescore::find_model;
use self::rescore::{ModelLoader, RescoreState};
use self::session::SessionInfo;
use qingjian_core::Engine;
use qingjian_platform::protocol::{ClientMessage, ServerMessage, SessionId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// 學習資料落盤間隔（與 macOS 殼一致）；Server 沒有定時器，借訊息節拍與主迴圈的 tick 看時間。
const LEARNING_FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// Fcitx5 輸入上下文分派器；所有 Engine 操作都在 Server 主執行緒序列執行。
pub struct Router {
    /// 唯一的詞庫及學習服務，輸入狀態在會話切換時交換。
    engine: Engine,

    /// 按鍵及候選展示配置。
    config: RouterConfig,

    /// 會話表。
    sessions: HashMap<SessionId, SessionInfo>,

    /// 當前裝入 Engine 的會話。
    focused: Option<SessionId>,

    /// 當前候選與 preedit。
    composed: Option<Composed>,

    /// 當前高亮的跨頁下標。
    highlight: usize,

    /// 本輪是否已主動移動候選。
    navigated: bool,

    /// 可選的整句提示；首版沒有云服務。
    sentence: Option<String>,

    /// 刪除候選等操作提示。
    notice: Option<String>,

    /// 全服務幀號遞增，關閉再開不會複用展示身份。
    display_revision: u64,

    /// 最近一次學習落盤的時刻。
    last_flush: Instant,

    /// 本地整句模型（`.qjm` 或三件套目錄）；沒有模型檔案為 `None`。
    model_path: Option<PathBuf>,

    /// 進行中的模型載入；載入完接到 Engine 上就清掉。
    model_loader: Option<ModelLoader>,

    /// 重排的防抖 / 輪詢進行態。
    rescore: RescoreState,
}

impl Router {
    pub fn new(engine: Engine, mut config: RouterConfig) -> Self {
        config.page_size = config.page_size.clamp(1, 9);
        config.cloud_slots = 0;
        Self {
            engine,
            config,
            sessions: HashMap::new(),
            focused: None,
            composed: None,
            highlight: 0,
            navigated: false,
            sentence: None,
            notice: None,
            display_revision: 0,
            last_flush: Instant::now(),
            model_path: None,
            model_loader: None,
            rescore: RescoreState::default(),
        }
    }
    /// 一條客戶端訊息；無需答覆的通知返回 None。
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
    /// 到點了：接上載入好的模型、推進重排、到點落盤學習（輸入停止後也不能一直不落盤）。主迴圈超時與外掛的 `Poll` 都會調。
    pub fn tick(&mut self) {
        self.attach_loaded_model();
        self.advance_rescoring();
        if self.last_flush.elapsed() >= LEARNING_FLUSH_INTERVAL {
            self.flush_learning();
        }
    }
    pub(super) fn full_width_for(&self, english: bool) -> bool {
        if english {
            self.config.english_full_width
        } else {
            self.config.full_width
        }
    }
}

impl Drop for Router {
    fn drop(&mut self) {
        let sessions = self.sessions.keys().copied().collect::<Vec<_>>();
        for session in sessions {
            self.close_session(session);
        }
        self.flush_learning();
    }
}
