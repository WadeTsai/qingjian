//! 平臺層共用的、與具體視窗系統無關的部分：配置檔案，以及將來 Core 與殼之間的協議型別。
//!
//! 這裡的型別必須可序列化：macOS / Linux 上 Core 與殼同進程，Windows 上 Core 在獨立
//! Server 程序，同一套型別兩邊都用。

pub mod code_tables;
mod config;
pub mod dirs;
mod error;
pub mod extra_dictionaries;
pub mod logs;
pub mod protocol;
pub mod resources;

pub use config::{
    Appearance, AppsConfig, AuxCodeConfig, CandidateRenderer, Config, DEFAULT_DOMAINS,
    DEFAULT_ENGLISH_CANDIDATES_ON, DEFAULT_ENGLISH_CANDIDATES_ON_LINUX,
    DEFAULT_ENGLISH_CANDIDATES_ON_MACOS, DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS, DEFAULT_PAGE_KEYS,
    DEFAULT_THEME, DictionariesConfig, FontSize, GeneralConfig, KeyCombo, LEARNING_LANGUAGE_OFF,
    LayoutMode, LocalModelConfig, LogLevel, MAX_PAGE_SIZE, Modifiers, PAGE_KEY_OPTIONS,
    PreeditMode, Scheme, ShiftLetter, ShortcutConfig, SwitchKey, SwitchKeys, UpdateChannel,
    UpdateConfig, scheme_label, scheme_name,
};
pub use error::ConfigError;
