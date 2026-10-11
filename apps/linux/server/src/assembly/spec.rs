//! Linux Engine 裝配所需的產品資料與使用者路徑。
use std::path::PathBuf;

use qingjian_core::Language;
use qingjian_platform::DictionariesConfig;

use super::LanguageModelFiles;

/// 裝配要用的資料檔案。除詞庫外都可選：缺哪個就少哪個功能。
pub struct AssemblySpec {
    /// 主詞庫（`.qj` 或 TSV）。
    pub dict: PathBuf,

    /// 學習語言的釋義表。
    pub glossary: Option<(Language, PathBuf)>,

    /// 英→中釋義表（英文候選的中文釋義）。
    pub english_glossary: Option<PathBuf>,

    /// 英文詞表。
    pub english: Option<PathBuf>,

    /// emoji 表（多張合成一張）。
    pub emoji: Vec<PathBuf>,

    /// 語言模型；沒有就退化成一元詞頻整句。
    pub language_model: Option<LanguageModelFiles>,

    /// 隨包領域詞庫目錄。
    pub bundled_dicts_dir: Option<PathBuf>,

    /// `[dictionaries]` 配置。
    pub dictionaries: DictionariesConfig,

    /// 詞彙等級表目錄（`levels-<語言>.tsv`）。
    pub levels_dir: Option<PathBuf>,

    /// 使用者資料目錄（XDG data home）；沒有就都只在記憶體。
    pub user_dir: Option<PathBuf>,

    /// 是否寫輸入日誌（`[general] input_log`）。
    pub input_log: bool,

    /// 輸入日誌目錄（XDG_STATE_HOME 下）；預設回落使用者資料目錄。
    pub log_dir: Option<PathBuf>,
}

impl AssemblySpec {
    pub fn new(dict: impl Into<PathBuf>) -> Self {
        Self {
            dict: dict.into(),
            glossary: None,
            english_glossary: None,
            english: None,
            emoji: Vec::new(),
            language_model: None,
            bundled_dicts_dir: None,
            dictionaries: DictionariesConfig::default(),
            levels_dir: None,
            user_dir: None,
            input_log: false,
            log_dir: None,
        }
    }
}
