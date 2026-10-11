//! 青簡 Windows 設定介面入口：左側導航欄 + 各分節表單，讀寫 `%APPDATA%\Qingjian\config.toml`。
//! UI 用 Windows Reactor；非 Windows 編成空殼，讓工作區能整體編譯。
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod log;
#[cfg(windows)]
mod panel;
#[cfg(windows)]
mod single_instance;

#[cfg(windows)]
fn main() {
    if !single_instance::acquire() {
        return;
    }
    if let Err(error) = windows_reactor::App::run_component::<panel::Settings>(()) {
        // GUI 子系統沒有控制台：記檔案日誌，再彈個框讓使用者知道發生了什麼（最常見是執行庫檔案缺失）。
        log::error(format!("設定介面啟動失敗: {error:?}"));
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("青簡設定")
            .set_description(format!(
                "設定介面啟動失敗，請重新安裝青簡；仍不行請把日誌目錄發給作者。\n\n{error}"
            ))
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("qingjian-settings 僅支援 Windows");
}
