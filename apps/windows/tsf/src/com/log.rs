//! DLL 自己的檔案日誌：按天一個檔案 `%LOCALAPPDATA%\Qingjian\logs\tsf.<YYYY-MM-DD>.log`，與 Server / 設定程式同目錄，
//! 只留最近 [`KEEP_DAYS`] 天（命名與清理規則在 `qingjian_platform::logs::daily`）。
//! 不走 tracing 全域性訂閱器（宿主程序可能已裝了自己的）；任何失敗都吞掉，日誌不能拖垮宿主。
//! 每次都開檔案追加一行：DLL 被載入進每個應用程序，多程序同時追加同一天的檔案，這樣最省事也最穩。
//! 日期用 `GetLocalTime` 而不是時區庫：這份程式碼跑在每一個有文本框的程序裡，越輕越好。

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use qingjian_platform::logs::daily::{self, KEEP_DAYS};
use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::System::SystemInformation::GetLocalTime;

const PREFIX: &str = "tsf";

/// 本程序上次寫日誌的日子（自 1970 起的天數）；換了天才清一次舊檔案。
static LAST_DAY: AtomicU32 = AtomicU32::new(0);

pub(crate) fn log(message: &str) {
    let Some(dir) = dir() else { return };
    let t = unsafe { GetLocalTime() };
    let day = day_number(&t);
    if LAST_DAY.swap(day, Ordering::Relaxed) != day {
        let _ = std::fs::create_dir_all(&dir);
        daily::prune(&dir, PREFIX, day, KEEP_DAYS);
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(file_name(&t)))
    else {
        return;
    };
    let _ = writeln!(file, "{} [pid {}] {message}", now(&t), std::process::id());
}

fn file_name(t: &SYSTEMTIME) -> String {
    daily::file_name(
        PREFIX,
        i64::from(t.wYear),
        u32::from(t.wMonth),
        u32::from(t.wDay),
    )
}

fn now(t: &SYSTEMTIME) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn dir() -> Option<PathBuf> {
    qingjian_platform::dirs::log_dir()
}

/// 自 1970-01-01 起的天數，日期比較用。
fn day_number(t: &SYSTEMTIME) -> u32 {
    daily::days_from_civil(i64::from(t.wYear), u32::from(t.wMonth), u32::from(t.wDay))
}
