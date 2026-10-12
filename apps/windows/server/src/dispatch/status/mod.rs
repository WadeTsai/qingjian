//! 全域性中英模式與懸浮狀態條：模式只有 Server 這一份，DLL 切了用 `ModeChanged` 報來，啟用 / 獲焦 / 輪詢時用
//! `SyncMode` 取走（取的同時說明青簡是當前輸入法，狀態條顯示）；切成別的輸入法時 DLL 發 `ImeSwitched` 收起。
//! 會話關閉（應用退出）不收——狀態條常駐桌面。狀態條上的點選經 [`StatusEvent`] 回到這裡：
//! 切模式直接改全域性模式，各 DLL 下一拍取走；切標點 / 拖動寫回配置檔案（熱載入會再讀回來）。

mod event;
mod sink;
mod view;

use qingjian_platform::protocol::IndicatorCommand;
use qingjian_platform::{Config, Scheme, scheme_label};

pub use self::event::StatusEvent;
pub use self::sink::{NoopStatusSink, StatusSink};
pub use self::view::StatusView;
use super::Router;

impl Router {
    /// DLL 那邊使用者切了模式：成為全域性模式。內建英文模式關著時不收英文。
    pub(super) fn handle_mode_changed(&mut self, english: bool) {
        self.english = english && self.config.english_mode;
        self.ime_active = true;
        self.reconcile_status();
    }

    /// 有 DLL 來取模式：青簡是當前輸入法。
    pub(super) fn handle_ime_active(&mut self) {
        if !self.config.english_mode {
            self.english = false;
        }
        if !self.ime_active {
            self.ime_active = true;
            self.reconcile_status();
        }
    }

    pub(super) fn handle_ime_switched(&mut self) {
        self.ime_active = false;
        self.reconcile_status();
    }

    /// 狀態條上的操作。
    pub fn handle_status_event(&mut self, event: StatusEvent) {
        match event {
            StatusEvent::ToggleMode => {
                // 關掉內建英文模式後這一格不切模式：DLL 那邊也會攔（配置改了沒切走再切回時兩邊都擋住）
                if !self.config.english_mode {
                    tracing::debug!("內建英文模式已關閉，狀態條不切模式");
                    return;
                }
                self.english = !self.english;
                tracing::debug!(english = self.english, "狀態條：切換中英模式");
            }
            StatusEvent::TogglePunctuation => {
                // 中英各記一份，切的是當前模式那份；還沒報過模式時按中文算。
                let english = self.english;
                let full_width = !self.full_width_for(english);
                let key = if english {
                    self.config.english_full_width = full_width;
                    "english_full_width_punctuation"
                } else {
                    self.config.full_width = full_width;
                    "full_width_punctuation"
                };
                tracing::debug!(english, full_width, "狀態條：切換全形標點");
                self.persist("general", key, full_width);
            }
            StatusEvent::Moved(x, y) => {
                self.config.status_pos = Some((x, y));
                self.persist("status_bar", "x", i64::from(x));
                self.persist("status_bar", "y", i64::from(y));
            }
        }
        self.reconcile_status();
    }

    /// 工作列圖示右鍵選單：標點與懸浮條的開關寫回配置檔案（熱載入會再讀回來），設定程式交給 UI 起。
    pub(super) fn handle_indicator(&mut self, command: IndicatorCommand) {
        match command {
            IndicatorCommand::TogglePunctuation => {
                self.handle_status_event(StatusEvent::TogglePunctuation);
            }
            IndicatorCommand::ToggleStatusBar => {
                self.config.status_enabled = !self.config.status_enabled;
                self.persist("status_bar", "enabled", self.config.status_enabled);
                self.reconcile_status();
            }
            IndicatorCommand::OpenSettings => self.status.open_settings(),
            IndicatorCommand::OpenDownload => self.status.open_download(),
        }
    }

    /// 寫回配置檔案一個鍵；沒有配置路徑（測試）就只改記憶體。
    fn persist(&self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        let Some(path) = self.config_path() else {
            return;
        };
        if let Err(error) = Config::set_value(path, section, key, value) {
            tracing::warn!(%error, section, key, "寫回配置失敗");
        }
    }

    /// 當前模式下標點轉不轉全形：中英各一份配置。
    pub(super) fn full_width_for(&self, english: bool) -> bool {
        if english {
            self.config.english_full_width
        } else {
            self.config.full_width
        }
    }

    /// 開著且青簡在前臺就顯示，否則收起。熱載入後也調一次。
    pub(super) fn reconcile_status(&mut self) {
        match self.ime_active.then_some(self.english) {
            Some(english) if self.config.status_enabled => {
                self.status.show_status(StatusView {
                    english,
                    zhuyin: self.scheme() == Scheme::Zhuyin,
                    // 現算，不存下來：存了會與 scheme / wubi 冗餘、手搓配置的地方就漂移；方案還看聚焦會話的 profile
                    scheme: Some(scheme_label(self.scheme(), self.config.wubi))
                        .filter(|label| !label.is_empty()),
                    full_width: self.full_width_for(english),
                    appearance: self.config.appearance,
                    anchor: self.config.status_pos,
                    mode: self.input_mode(),
                });
            }
            _ => self.status.hide_status(),
        }
    }
}
