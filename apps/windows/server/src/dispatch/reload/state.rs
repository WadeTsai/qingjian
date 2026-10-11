//! 配置熱載入記的狀態。

use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use qingjian_core::Language;
use qingjian_dictionary::Dictionary;
use qingjian_platform::{
    AuxCodeConfig, DictionariesConfig, UpdateConfig, code_tables, extra_dictionaries,
};
use qingjian_predict::PredictConfig;

/// 隨包與使用者資料目錄：啟動與熱載入用的是同一批（詞庫、碼錶）。
/// 分開傳引數會越傳越長，且熱載入與原路徑不一致時找不到檔案。
#[derive(Debug, Clone, Default)]
pub struct DataDirs {
    /// 使用者資料根目錄（個人釋義表在它下面；`dicts/` / `codes/` 的父目錄）。
    pub user_root: Option<PathBuf>,

    /// 隨包領域詞庫目錄。
    pub bundled_dicts: Option<PathBuf>,

    /// 隨包輔碼碼錶目錄（隨包根 `data/generated/codes/`，與 `dicts/` 並列）。
    pub bundled_codes: Option<PathBuf>,

    /// 使用者匯入詞庫目錄（`<使用者目錄>/dicts`）。
    pub user_dicts: Option<PathBuf>,

    /// 使用者匯入碼錶目錄（`<使用者目錄>/codes`）。
    pub user_codes: Option<PathBuf>,
}

impl DataDirs {
    /// 使用者 `codes/` 的逐檔案快照（路徑、mtime、長度）；沒配目錄為空。啟動與熱載入輪詢共用。
    pub(super) fn code_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        self.user_codes
            .as_deref()
            .map(code_tables::snapshot)
            .unwrap_or_default()
    }

    /// 使用者 `dicts/` 的同一份快照；沒配目錄為空。
    pub(super) fn dict_snapshot(&self) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
        self.user_dicts
            .as_deref()
            .map(extra_dictionaries::snapshot)
            .unwrap_or_default()
    }
}

/// 熱載入狀態。
pub(crate) struct ConfigReload {
    /// `config.toml` 路徑。
    pub(super) config_path: PathBuf,

    /// 上次看檔案的時間（節流用）。
    pub(super) last_check: Instant,

    /// 隨包資料根目錄（釋義表在 `data/generated` 下）。
    pub(super) root: PathBuf,

    /// 隨包與使用者資料目錄。
    pub(super) dirs: DataDirs,

    /// 上次看到的碼錶檔案快照（逐檔案 mtime + 長度）：設定頁剛匯入一張表、或手工拷進半個
    /// `.qj` 後補全，都靠它即時生效。
    pub(super) code_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

    /// 上次看到的 mtime。
    pub(super) last_mtime: Option<SystemTime>,

    /// 已應用的 `[predict]`。
    pub(super) applied_predict: PredictConfig,

    /// 已應用的 `[dictionaries]`。
    pub(super) applied_dictionaries: DictionariesConfig,

    /// 已應用的 `[aux_code]`。
    pub(super) applied_aux_code: AuxCodeConfig,

    /// 已應用的學習語言（`None` 為關）。
    pub(super) applied_language: Option<Language>,

    /// 最近載入的使用者詞庫檔案快照（路徑、修改時間、長度）。
    pub(super) dictionary_files: Vec<(PathBuf, Option<SystemTime>, u64)>,

    /// 當前的 `[update]`。
    pub(super) update: UpdateConfig,

    /// 檢查更新：結果寫進使用者目錄的 `update.json`，設定程式的「關於」頁讀它；拿不到使用者目錄時沒有。
    pub(super) updates: Option<qingjian_update::Checker>,
}

impl ConfigReload {
    /// 按上次有效配置裝配詞庫，不把使用者目錄中的學習資料當作詞庫。
    pub(super) fn load_dictionaries(&self) -> Vec<Dictionary> {
        let dictionaries = extra_dictionaries::load(
            self.dirs.bundled_dicts.as_deref(),
            self.dirs.user_dicts.as_deref(),
            &self.applied_dictionaries,
        );
        tracing::info!(count = dictionaries.len(), "附加词库已热重装");
        dictionaries
    }
}
