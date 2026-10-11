//! 按天一個檔案的日誌：檔名約定 `<字首>.<YYYY-MM-DD>.log` 與清理舊檔案。
//!
//! Server 用 tracing_appender 滾動，TSF DLL 與設定程式自己開檔案追加；三者共用這裡的命名與「只留最近幾天」規則，
//! 日期怎麼取由呼叫方定（DLL 用 `GetLocalTime`，不想在每個應用程序裡多拉一份時區庫）。

use std::path::Path;

/// 預設保留天數，三個程序一致。
pub const KEEP_DAYS: u32 = 7;

/// `<字首>.<YYYY-MM-DD>.log`。
pub fn file_name(prefix: &str, year: i64, month: u32, day: u32) -> String {
    format!("{prefix}.{year:04}-{month:02}-{day:02}.log")
}

/// 公曆日期 → 自 1970-01-01 起的天數（Howard Hinnant 的 `days_from_civil`），夠比大小用。
pub fn days_from_civil(year: i64, month: u32, day: u32) -> u32 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u32;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + i64::from(doe) - 719_468) as u32
}

/// 刪掉目錄裡早於 `today - keep_days` 的 `<字首>.<日期>.log`；別的檔案（包括其他程序的日誌）不碰。
pub fn prune(dir: &Path, prefix: &str, today: u32, keep_days: u32) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(day) = name.to_str().and_then(|name| log_day(name, prefix)) else {
            continue;
        };
        if today.saturating_sub(day) >= keep_days {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// `tsf.2026-09-11.log` → 那天的天數；不是該字首的日誌檔案為 `None`。
pub fn log_day(name: &str, prefix: &str) -> Option<u32> {
    let date = name
        .strip_prefix(prefix)?
        .strip_prefix('.')?
        .strip_suffix(".log")?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_numbers_follow_the_calendar() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(
            days_from_civil(2026, 9, 11) - days_from_civil(2026, 9, 4),
            7
        );
        assert_eq!(
            days_from_civil(2026, 3, 1) - days_from_civil(2026, 2, 28),
            1
        );
    }

    #[test]
    fn only_dated_logs_of_the_prefix_are_recognised() {
        assert_eq!(
            log_day("tsf.2026-09-11.log", "tsf"),
            Some(days_from_civil(2026, 9, 11))
        );
        assert_eq!(log_day("tsf.log", "tsf"), None);
        assert_eq!(log_day("server.2026-09-11.log", "tsf"), None);
        assert_eq!(log_day("tsf.2026-13-01.log", "tsf"), None);
        assert_eq!(
            file_name("settings", 2026, 9, 12),
            "settings.2026-09-12.log"
        );
    }

    #[test]
    fn prune_keeps_recent_and_foreign_files() {
        let dir = std::env::temp_dir().join(format!("qingjian-daily-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let today = days_from_civil(2026, 9, 12);
        for name in [
            "tsf.2026-09-12.log",
            "tsf.2026-09-06.log",
            "tsf.2026-09-05.log",
            "server.2026-09-01.log",
            "notes.txt",
        ] {
            std::fs::write(dir.join(name), "").unwrap();
        }
        prune(&dir, "tsf", today, KEEP_DAYS);
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "notes.txt",
                "server.2026-09-01.log",
                "tsf.2026-09-06.log",
                "tsf.2026-09-12.log"
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
