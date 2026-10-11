//! 程序級單例：一個 Engine + 一個候選視窗，所有輸入會話共用。
//!
//! IMK 的所有回撥都在主執行緒，所以用 `thread_local` 而不是鎖；從別的執行緒訪問會拿到 `None`。
//!
//! 配置只有一條通路：[`Host::apply_config`] 把當前 `Config` 推給 Engine 與介面。啟動、選單開關、
//! 設定視窗、手改檔案被監視到，全都走它；三個入口都只寫 `config.toml`，不各存一套狀態。

mod cloud;
mod config;
mod diagnostics;
mod dictionaries;
mod init;
mod model;
mod presenting;
mod session;
mod settings;

use std::cell::RefCell;
use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::{NSProcessInfo, NSRect, NSString};
use qingjian_core::{
    Candidate, CandidateKind, Cell, CloudWord, EmojiTable, Engine, FuzzyRules, Language, ModeKeys,
    NoGlossFiller, NoInputLogger, NoPredictor, NoTranslator, Prediction,
};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, InputLog, UsageStats, VocabularyBook};
use qingjian_lm::BigramModel;
use qingjian_platform::extra_dictionaries;
use qingjian_platform::{
    Appearance, AppsConfig, CandidateRenderer, DEFAULT_ENGLISH_CANDIDATES_ON, DictionariesConfig,
    GeneralConfig, KeyCombo, LEARNING_LANGUAGE_OFF, LayoutMode, LocalModelConfig, LogLevel,
    Modifiers, PAGE_KEY_OPTIONS, PreeditMode, Scheme, ShortcutConfig, UpdateChannel,
};
use qingjian_predict::{
    CloudGlossFiller, CloudPredictor, ConnectionTest, PredictConfig, PredictError,
};
use qingjian_translate::{Glossary, LayeredTranslator, LevelTable, PersonalGlossary};

use crate::app::BundleInfo;
use crate::app::{Settings, logging, paths};
use crate::candidates::{CandidateWindow, Frame, Preedit, Row};
use crate::error::HostError;
use crate::menubar::{InputMenu, MenuAction, ModeIndicator};
use crate::preferences::{PreferencesWindow, Setting, SettingValue, UpdateStatus};

use cloud::{CloudTestMonitor, PredictMonitor};
use config::{ConfigWatch, TextReplacement};
pub use dictionaries::DictionaryInfo;
pub use init::init;
use model::RescoreMonitor;
use presenting::Notice;
pub use presenting::TranslationJob;
pub use session::Session;

pub struct Host {
    /// 輸入核心。平臺層只能通過它的公開 API 拿候選，不允許碰詞庫或排序。
    pub engine: Engine,

    /// 候選視窗。
    pub window: CandidateWindow,

    /// 選單欄的中 / 英狀態項。
    pub indicator: ModeIndicator,

    /// 輸入法選單，掛在狀態項和系統輸入源選單上。
    pub menu: InputMenu,

    /// 偏好設定視窗。
    pub preferences: PreferencesWindow,

    /// 配置檔案的當前值與修改時間。
    pub settings: Settings,

    /// 配置檔案監視定時器，啟用期間跑。
    pub watch: ConfigWatch,

    /// 內建主題與使用者主題目錄裡的主題；啟用期間隨配置定時器一起看目錄有沒有變。
    pub themes: qingjian_render::ThemeLibrary,

    /// 上次把學習資料落盤的時間；啟用期間的定時器按 [`LEARNING_FLUSH_INTERVAL`] 再刷一次。
    pub last_flush: std::time::Instant,

    /// 當前 Predictor 是按哪份 `[predict]` 建的；配置沒變就不重建（重建會起新執行緒、丟快取）。
    applied_predict: PredictConfig,

    /// 附加詞庫是按哪份 `[dictionaries]` 裝的；開關變了才重新載入。
    applied_dictionaries: DictionariesConfig,

    /// 偏好設定「詞庫」頁顯示的列表，勾選框 / 移除按鈕的下標對著它。
    dictionary_list: Vec<DictionaryInfo>,

    /// 當前學習語言；`None` 為關（不顯示譯文）。
    learning_language: Option<Language>,

    /// 打進包裡的釋義表語言，設定視窗按這個順序列。
    languages: Vec<Language>,

    /// 版本號與構建標識，診斷資訊裡用。
    version: String,

    /// 見 [`BundleInfo::build`]。
    build: String,

    /// 每頁候選數（配置 `[general] page_size`，已夾到 1–9）。
    pub page_size: usize,

    /// 候選視窗第一頁末尾留給雲端詞的格數（配置 `[predict] slots`）。
    pub cloud_slots: usize,

    /// 翻頁鍵對（上一頁、下一頁）。
    pub page_keys: (char, char),

    /// 組句中敲會轉全形的標點先把高亮候選上屏、再補該標點（配置 `[general] punct_commits`，預設關）。
    pub punct_commits: bool,

    /// 配數字鍵上屏第一 / 第二個譯詞的修飾鍵組合（配置 `[shortcut] translation` / `translation_second`）。
    pub translation_keys: (Modifiers, Modifiers),

    /// 配數字鍵刪候選的修飾鍵（配置 `[shortcut] delete_candidate`）。
    pub delete_keys: Modifiers,

    /// 候選視窗頂行顯示的一句臨時狀態（刪了什麼詞），下一次查詢就沒了。
    pub status: Option<String>,

    /// 輸入日誌是否在記（配置 `[general] input_log`），換了才重開檔案。
    input_log_enabled: Option<bool>,

    /// 翻譯選中文字的快捷鍵（配置 `[shortcut] translate_selection`）。
    pub translate_keys: KeyCombo,

    /// 進行中的「翻譯選中文字」；有它時候選視窗顯示的是譯文（或「翻譯中…」），按鍵先歸它處理。
    pub translation: Option<TranslationJob>,

    /// 正在顯示的提示（候選窗口裡一行字，幾秒後自動收）。
    pub notice: Option<Notice>,

    /// 組句中的拼音顯示在行內、候選視窗還是兩處。
    pub preedit_mode: PreeditMode,

    /// 候選視窗豎排 / 橫排（配置 `[general] layout`）。
    pub layout: LayoutMode,

    /// 橫排時上下鍵展開成矩陣的開關（配置 `[general] horizontal_grid`，預設關）。
    pub horizontal_grid: bool,

    /// 英文模式是否給英文候選（配置 `[general] english_candidates`）。
    pub english_candidates: bool,

    /// 上次從系統讀到的文本替換（啟用輸入法時重讀），`[general] system_text_replacements` 開著時並進自定義短語。
    text_replacements: Vec<TextReplacement>,

    /// 按應用的行為（配置 `[apps]`）：哪些應用裡英文模式不給候選。
    pub apps: AppsConfig,

    /// 聯想結果輪詢定時器。
    pub monitor: PredictMonitor,

    /// 進行中的雲服務連通性測試（「雲服務」頁「測試連線」按鈕）；沒在測為 `None`。
    cloud_test: Option<ConnectionTest>,

    /// 連通性測試的輪詢定時器。
    cloud_test_monitor: CloudTestMonitor,

    /// 本地整句模型的防抖與輪詢定時器。
    rescore: RescoreMonitor,

    /// 正在後臺載入的模型；載入完接到 Engine 上就清掉。
    model_loader: Option<
        std::sync::mpsc::Receiver<
            Result<Box<dyn qingjian_core::sentence::SentenceScorer>, qingjian_neural::NeuralError>,
        >,
    >,

    /// 上次套用的 `[model]`，變了才過載 / 解除安裝。
    applied_model: Option<LocalModelConfig>,

    /// 檢查更新；拿不到資料目錄時沒有。
    updates: Option<qingjian_update::Checker>,

    /// 選單與「關於」頁上正顯示的更新狀態，變了才刷介面。
    update_status: UpdateStatus,

    /// 當前會話的候選、高亮、頁碼、preedit。
    pub session: Session,

    /// 組句中到達的整句補全，Tab 接受。
    pub sentence: Option<String>,

    /// 最近一次繪製時的游標矩形，聯想結果到達後在同一位置重畫。
    pub anchor: NSRect,
}

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// 啟用期間學習資料最多隔這麼久落一次盤。
const LEARNING_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// 輸入統計檔名，與學習資料同目錄（按天一行，見 `qingjian-learning::UsageStats`）。
const USAGE_FILE: &str = "usage.tsv";

/// 檢查更新的結果檔名，與學習資料同目錄（見 `qingjian-update::UpdateState`）。
const UPDATE_STATE_FILE: &str = "update.json";

/// 詞彙記錄檔名，與學習資料同目錄（一個譯詞一行，見 `qingjian-learning::VocabularyBook`）。
const VOCABULARY_FILE: &str = "user-vocab.tsv";

/// 可能打進包裡的釋義表語言，按這個順序在設定裡列出；檔案不存在的不列。
const GLOSSARY_LANGUAGES: [Language; 4] = [
    Language::English,
    Language::Japanese,
    Language::Spanish,
    Language::Vietnamese,
];

/// 在單例上執行操作。未初始化、不在主執行緒、或正處在另一次 `with` 之內（重入）時返回 `None`。
///
/// 重入是真會發生的：閉包裡若碰了應用那邊的東西（讀上下文、取游標位置、插入文字），IMK 會在等應用回話時
/// 跑一輪 run loop，`deactivateServer:` 這類回撥就可能在這時進來；這時再 `borrow_mut` 就是 panic 加程序退出。
/// 所以借不到就記一條日誌放過這次呼叫，呼叫方本來就都按 `None` 處理；根治靠把 IPC 放在借用之外。
pub fn with<R>(f: impl FnOnce(&mut Host) -> R) -> Option<R> {
    HOST.with(|host| match host.try_borrow_mut() {
        Ok(mut guard) => guard.as_mut().map(f),
        Err(_) => {
            tracing::warn!("Host 正在被借用（IMK 回调重入），本次调用跳过");
            None
        }
    })
}
