//! 輸入法程序沒有終端，日誌只寫檔案：`~/Library/Logs/Qingjian/qingjian.log.<日期>`。
//!
//! 按天分檔案，只留最近 [`KEEP_DAYS`] 天；檔案被使用者刪掉後下一條日誌會重新建（`tracing_appender::rolling`
//! 一直握著舊檔案描述符，刪掉後日志會寫進已經不在目錄裡的 inode，看起來就是「日誌檔案始終不出現」）。

mod log_file;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use qingjian_platform::LogLevel;
use qingjian_platform::logs::secrets::MaskingWriter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry, reload};

pub use log_file::LogFile;

/// 日誌檔名字首，後面跟 `.YYYY-MM-DD`。
pub const FILE_PREFIX: &str = "qingjian.log";

/// 保留最近幾天的日誌。
pub const KEEP_DAYS: i32 = 7;

/// 執行中改級別用的控制代碼（配置 `[general] log_level` 熱切換）。
static FILTER: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();

/// 環境變數 `RUST_LOG` 給了過濾器就以它為準，配置裡的級別不再生效（開發時用）。
static ENV_OVERRIDE: AtomicBool = AtomicBool::new(false);

/// 返回的 guard 要活到程序結束，否則檔案日誌會丟尾巴。啟動時按 info 記，配置載入後再按配置切。
pub fn init() -> Option<WorkerGuard> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    prune(&dir, jiff::Zoned::now().date());
    let (writer, guard) = tracing_appender::non_blocking(MaskingWriter::new(LogFile::new(dir)));
    let from_env = EnvFilter::try_from_default_env().ok();
    ENV_OVERRIDE.store(from_env.is_some(), Ordering::Relaxed);
    let (filter, handle) =
        reload::Layer::new(from_env.unwrap_or_else(|| filter_for(LogLevel::Info)));
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false),
        )
        .init();
    let _ = FILTER.set(handle);
    Some(guard)
}

/// 切日誌級別。`RUST_LOG` 環境變數在時不動。
pub fn set_level(level: LogLevel) {
    if ENV_OVERRIDE.load(Ordering::Relaxed) {
        return;
    }
    if let Some(handle) = FILTER.get()
        && let Err(error) = handle.reload(filter_for(level))
    {
        tracing::warn!(%error, "切换日志级别失败");
    }
}

/// 各級別的過濾器：debug 下網路庫仍只記 info，否則一次雲聯想刷幾十行。
fn filter_for(level: LogLevel) -> EnvFilter {
    match level {
        LogLevel::Info => EnvFilter::new("info"),
        LogLevel::Debug => {
            EnvFilter::new("debug,hyper=info,hyper_util=info,reqwest=info,h2=info,rustls=info")
        }
    }
}

/// 日誌目錄，選單「開啟日誌目錄」也用。
pub fn log_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Logs/Qingjian"))
}

/// 把日誌目錄裡的檔案加 `config.toml` 打成桌面上的 `qingjian-logs-<日期>.zip`（`zip -j`，不帶目錄層級），返回 zip 路徑。
/// 與 Windows 設定程式的「打包日誌到桌面」對應；金鑰在 `.env` 裡，不進包。
pub fn export_logs() -> Result<PathBuf, String> {
    let dir = log_dir().ok_or("找不到日志目录")?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|error| format!("读不到日志目录：{error}"))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .collect();
    files.extend(super::paths::config_file().filter(|path| path.is_file()));
    if files.is_empty() {
        return Err("没有可打包的日志".to_owned());
    }
    let desktop = PathBuf::from(std::env::var_os("HOME").ok_or("找不到主目录")?).join("Desktop");
    let zip = desktop.join(format!(
        "qingjian-logs-{}.zip",
        jiff::Zoned::now().strftime("%Y-%m-%d")
    ));
    let _ = std::fs::remove_file(&zip);
    let status = std::process::Command::new("/usr/bin/zip")
        .arg("-jq")
        .arg(&zip)
        .args(&files)
        .status()
        .map_err(|error| format!("起不了 zip：{error}"))?;
    if !status.success() {
        return Err(format!("zip 退出码 {status}"));
    }
    tracing::warn!(zip = %zip.display(), "用户导出日志");
    Ok(zip)
}

/// 刪掉目錄裡日期早於 `today - KEEP_DAYS` 的日誌檔案；檔名解析不出日期的不動。
pub fn prune(dir: &Path, today: jiff::civil::Date) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let Ok(oldest_kept) = today.checked_sub(jiff::Span::new().days(KEEP_DAYS - 1)) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(date) = name
            .to_str()
            .and_then(|n| n.strip_prefix(FILE_PREFIX))
            .and_then(|rest| rest.strip_prefix('.'))
            .and_then(|date| date.parse::<jiff::civil::Date>().ok())
        else {
            continue;
        };
        if date < oldest_kept {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_keeps_recent_files_and_unrelated_names() {
        let dir = std::env::temp_dir().join("qingjian-log-prune-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in [
            "qingjian.log.2026-09-04",
            "qingjian.log.2026-08-29",
            "qingjian.log.2026-08-28",
            "qingjian.log.bogus",
            "notes.txt",
        ] {
            std::fs::write(dir.join(name), "x").unwrap();
        }
        prune(&dir, jiff::civil::date(2026, 9, 4));
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "notes.txt",
                "qingjian.log.2026-08-29",
                "qingjian.log.2026-09-04",
                "qingjian.log.bogus"
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
