//! 設定視窗根元件：左側導航 + 右側當前分節頁。每改一項就原地寫回 `config.toml`（保留註釋）再重讀，
//! 介面始終反映檔案內容；Server 每秒看 mtime 熱載入。
//! 狀態在這裡，訊息在 [`message`]，生命週期在 [`component`]，表單零件在 [`controls`]，各頁在 [`pages`]。

mod cloud_status;
mod component;
mod controls;
mod message;
mod notice;
mod pages;
mod recorder;

use std::path::{Path, PathBuf};

use qingjian_platform::Config;
use qingjian_render::ThemeLibrary;
use windows_reactor::*;

use self::cloud_status::CloudStatus;
pub(crate) use self::message::Message;
use self::notice::Notice;
use self::pages::{
    about, advanced, aux_code, candidates, cloud, dictionaries, fuzzy, general, shortcut, usage,
};
use self::recorder::Recorder;

/// 左側標籤固定寬度，讓各行控制元件對齊。
const LABEL_WIDTH: f64 = 140.0;

/// 設定視窗狀態。
pub(crate) struct Settings {
    /// 當前配置，每次改動後從盤上重讀。
    pub(super) config: Config,

    /// `config.toml` 路徑。
    path: PathBuf,

    /// 當前導航分節 tag。
    page: String,

    /// 雲服務「測試連線」的狀態。
    cloud_status: CloudStatus,

    /// 「輔碼」頁的觸發鍵錄製狀態。
    recorder: Recorder,

    /// 觸發鍵錄製框（密碼框：不走輸入法，按 A–Z 直接進字元、不彈輸入法候選窗）：
    /// 進了錄製態把焦點交給它，使用者不用再點一下。
    record_box: ElementRef<PasswordBox>,

    /// 頁面底部的臨時提示（匯入統計 / 失敗原因）。
    notice: Notice,

    /// Server 或「立即檢查」落盤的檢查更新結果（使用者目錄的 `update.json`）。
    update_state: qingjian_update::UpdateState,

    /// 「立即檢查」正在跑 / 剛失敗的原因。
    update_checking: bool,
    update_error: Option<String>,

    /// 最近一次詞庫操作的結果，顯示在詞庫頁。
    dictionary_status: String,

    /// 系統裡的字族名（DirectWrite），「字型」框的提示用。
    families: Vec<String>,

    /// 「字型」框里正在敲的文字；`None` 顯示配置裡的值。
    font_query: Option<String>,
}

impl Settings {
    /// `%APPDATA%\Qingjian\config.toml`；取不到 `APPDATA` 退回工作目錄。
    fn config_path() -> PathBuf {
        qingjian_platform::dirs::config_path().unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    /// 檢查更新的結果檔案 `%APPDATA%\Qingjian\update.json`（Server 寫，這裡讀）。
    fn update_state_path() -> Option<PathBuf> {
        qingjian_platform::dirs::user_dir().map(|dir| dir.join("update.json"))
    }

    /// 配置檔案不在就寫出模板：這個賬戶下 Server 還沒跑過時，儲存與「在記事本中開啟」都要有檔案。
    fn ensure_config_file(path: &Path) {
        if let Err(error) = Config::write_template_if_missing(path) {
            crate::log::warn(format!("寫配置模板失敗: {error}"));
        }
    }

    /// 資料目錄 `%APPDATA%\Qingjian`。
    fn data_dir(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    /// 落盤一個配置值再重讀。失敗只打印。
    fn save(&mut self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        if let Err(error) = Config::set_value(&self.path, section, key, value) {
            crate::log::warn(format!("儲存 [{section}] {key} 失敗: {error}"));
            return;
        }
        self.reload();
    }

    /// 字號框（`style` 是 `candidate` / `annotation`）：清空或填 0 回到主題的，不限範圍；
    /// 沒設過時框裡顯示主題的字號，原樣交回不寫（見 [`qingjian_platform::FontSize::edited`]）。
    fn save_font_size(&mut self, style: &str, size: Option<f64>) {
        let current = match style {
            "candidate" => self.config.general.candidate_font_size,
            _ => self.config.general.annotation_font_size,
        };
        let themes = ThemeLibrary::load(qingjian_platform::dirs::themes_dir().as_deref());
        let theme_size = themes
            .resolve(self.config.general.theme_id(), false)
            .theme_size(style);
        if let Some(size) = current.edited(size.filter(|size| !size.is_nan()), theme_size) {
            self.save("general", &format!("{style}_font_size"), size);
        }
    }

    /// 落盤一個字串陣列再重讀。
    fn save_array(&mut self, section: &str, key: &str, values: &[String]) {
        if let Err(error) = Config::set_array(&self.path, section, key, values) {
            crate::log::warn(format!("儲存 [{section}] {key} 失敗: {error}"));
            return;
        }
        self.reload();
    }

    fn reload(&mut self) {
        if let Ok(config) = Config::load(&self.path) {
            self.config = config;
        }
    }

    fn page_content(&self, context: &mut ViewContext<Self>) -> View {
        match self.page.as_str() {
            "candidates" => candidates::view(self, context),
            "shortcut" => shortcut::view(self, context),
            "cloud" => cloud::view(self, context),
            "fuzzy" => fuzzy::view(self, context),
            "dictionaries" => dictionaries::view(self, context),
            "aux_code" => aux_code::view(self, context),
            "usage" => usage::view(self, context),
            "advanced" => advanced::view(self, context),
            "about" => about::view(self, context),
            _ => general::view(self, context),
        }
    }
}
