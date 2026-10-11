//! 設定程式自己的檔案日誌：`%LOCALAPPDATA%\Qingjian\logs\settings.<YYYY-MM-DD>.log`，與 Server / DLL 同目錄。
//! exe 是 GUI 子系統沒有控制台，`eprintln!` 的字全丟；啟動失敗、儲存配置失敗這些得有地方看。
//! 與 DLL 一樣開檔案追加一行、失敗吞掉，只留最近 7 天（命名與清理在 `qingjian_platform::logs::daily`）。

use std::fmt::Display;
use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};

use qingjian_platform::logs::daily::{self, KEEP_DAYS};

const PREFIX: &str = "settings";

/// 上次寫日誌的日子（自 1970 起的天數）；換了天才清一次舊檔案。
static LAST_DAY: AtomicU32 = AtomicU32::new(0);

pub(crate) fn warn(message: impl Display) {
    write("WARN", &message.to_string());
}

pub(crate) fn error(message: impl Display) {
    write("ERROR", &message.to_string());
}

fn write(level: &str, message: &str) {
    let Some(dir) = qingjian_platform::dirs::log_dir() else {
        return;
    };
    let now = jiff::Zoned::now();
    let (year, month, day) = (
        i64::from(now.year()),
        u32::from(now.month().unsigned_abs()),
        u32::from(now.day().unsigned_abs()),
    );
    let today = daily::days_from_civil(year, month, day);
    if LAST_DAY.swap(today, Ordering::Relaxed) != today {
        let _ = std::fs::create_dir_all(&dir);
        daily::prune(&dir, PREFIX, today, KEEP_DAYS);
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(daily::file_name(PREFIX, year, month, day)))
    else {
        return;
    };
    let _ = writeln!(
        file,
        "{} {level} [pid {}] {}",
        now.strftime("%Y-%m-%d %H:%M:%S%.3f"),
        std::process::id(),
        qingjian_platform::logs::secrets::mask(message)
    );
}
