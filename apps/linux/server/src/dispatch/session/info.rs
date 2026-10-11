//! 單個 Fcitx 輸入上下文儲存的輸入狀態。
use crate::dispatch::composed::Composed;
use qingjian_core::EngineSession;

pub(crate) struct SessionInfo {
    /// 應用標識。
    pub(crate) app: Option<String>,

    /// 新 Linux 客戶端已協商顯示回報。
    pub(crate) display_identity: Option<crate::protocol::DisplayIdentity>,

    /// 待確認的當前候選幀。
    pub(crate) display_frame: Option<qingjian_platform::protocol::Frame>,

    /// 上次發給外掛的幀；外掛定時 Poll 時內容沒變就沿用展示身份，不算新的展示。
    pub(crate) last_frame: Option<qingjian_platform::protocol::Frame>,

    /// Server 持有的中英模式與單擊 Shift 狀態。
    pub(crate) english: bool,

    pub(crate) shift_pending: bool,

    /// 最近的框架能力，首次報告前為未知。
    pub(crate) capabilities: Option<crate::protocol::Capabilities>,

    pub(crate) disabled: bool,

    /// 焦點事實；只有當前聚焦幀允許展示回報。
    pub(crate) active: bool,

    /// 新會話預設私密，收到能力通知後才允許學習。
    pub(crate) private: bool,

    /// 掛起的 Engine 輸入狀態。
    pub(crate) engine: EngineSession,

    /// 掛起的候選列表。
    pub(crate) composed: Option<Composed>,

    /// 高亮下標。
    pub(crate) highlight: usize,

    /// 本輪是否移動候選。
    pub(crate) navigated: bool,
}
impl SessionInfo {
    pub(crate) fn new(app: Option<String>) -> Self {
        Self {
            app,
            english: false,
            shift_pending: false,
            capabilities: None,
            disabled: false,
            active: true,
            display_identity: None,
            display_frame: None,
            last_frame: None,
            private: true,
            engine: EngineSession::default(),
            composed: None,
            highlight: 0,
            navigated: false,
        }
    }
}
