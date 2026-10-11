//! Windows 上使用者目錄的約定，Server / TSF DLL / 設定程式三處共用：
//!
//! - **資料** `%APPDATA%\Qingjian`：配置、金鑰、學習資料、統計、輸入日誌（漫遊目錄，跟著賬戶走）。
//! - **日誌** `%LOCALAPPDATA%\Qingjian\logs`：三個程序的執行日誌都在這一個目錄裡，
//!   `server.<日期>.log` / `tsf.<日期>.log` / `settings.<日期>.log`，使用者反饋問題時整個目錄打包即可。
//!   放本機目錄有兩個原因：日誌本來就是這臺機器的東西，不該跟賬戶漫遊；DLL 被載入進
//!   AppContainer 應用（工作列搜尋 / 設定）時寫不了漫遊目錄，Server 啟動時會給這個目錄授權。
//!
//! 只查環境變數、不碰系統 API，其他平臺也能編（拿不到為 `None`）。macOS 有自己的 `paths.rs`，不走這裡。

use std::path::PathBuf;

/// 使用者資料目錄 `%APPDATA%\Qingjian`。
pub fn user_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join("Qingjian"))
}

/// 配置檔案 `%APPDATA%\Qingjian\config.toml`。
pub fn config_path() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("config.toml"))
}

/// 使用者主題目錄 `%APPDATA%\Qingjian\themes`，一個主題一個子目錄（`<id>\theme.json`），不負責建立。
pub fn themes_dir() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("themes"))
}

/// 執行日誌目錄 `%LOCALAPPDATA%\Qingjian\logs`，不負責建立。
pub fn log_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Qingjian").join("logs"))
}
