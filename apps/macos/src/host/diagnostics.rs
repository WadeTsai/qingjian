//! 診斷與日誌：「關於」頁的診斷資訊（金鑰抹掉）、輸入日誌開關與清空、開啟外部程式。

use super::*;

impl Host {
    /// 給作者排查問題用的環境摘要：版本、系統、載入的資料、配置原文（金鑰抹掉）、日誌目錄。
    pub(super) fn diagnostics(&self) -> String {
        use std::fmt::Write as _;

        let mut out = String::new();
        let _ = writeln!(out, "青简 {} ({})", self.version, self.build);
        let os = NSProcessInfo::processInfo().operatingSystemVersionString();
        let _ = writeln!(out, "macOS {os} · {}", std::env::consts::ARCH);
        let _ = writeln!(out, "主词库：{} 条", self.engine.dictionary().len());
        for info in &self.dictionary_list {
            let state = if info.broken {
                "坏文件"
            } else if info.enabled {
                "启用"
            } else {
                "关闭"
            };
            let kind = if info.builtin {
                "随包词库"
            } else {
                "导入词库"
            };
            let _ = writeln!(out, "{kind}：{} · {} 条 · {state}", info.name, info.entries);
        }
        let languages: Vec<&str> = self.languages.iter().map(|l| l.code()).collect();
        let _ = writeln!(out, "释义表：{}", languages.join(" "));
        let _ = writeln!(
            out,
            "输入日志：{}",
            match Self::input_log_path() {
                Some(path) if self.input_log_enabled == Some(true) =>
                    format!("开，{} 行", InputLog::line_count(&path)),
                _ => "关".to_owned(),
            }
        );
        let usage = self.engine.usage_summary();
        let _ = writeln!(
            out,
            "输入统计：累计 {} 字 · {} 天",
            usage.total.hanzi, usage.days
        );
        let vocabulary = self.engine.vocabulary_summary();
        let _ = writeln!(
            out,
            "词汇（{}）：见过 {} · 看熟 {} · 上屏过 {}",
            self.learning_language
                .map_or("off", |language| language.code()),
            vocabulary.seen,
            vocabulary.familiar,
            vocabulary.committed
        );
        let _ = writeln!(
            out,
            "云联想：{}",
            if self.engine.prediction_enabled() {
                "开"
            } else {
                "关"
            }
        );
        if let Some(error) = self.settings.error() {
            let _ = writeln!(out, "配置错误：{error}");
        }
        if let Some(dir) = logging::log_dir() {
            let _ = writeln!(out, "日志目录：{}", dir.display());
        }
        if let Some(path) = self.settings.path() {
            let _ = writeln!(out, "配置文件：{}", path.display());
            if let Ok(text) = std::fs::read_to_string(path) {
                out.push_str("--- config.toml ---\n");
                out.push_str(&redact_secrets(&text));
            }
        }
        out
    }

    /// 輸入日誌檔案的路徑（資料目錄下 `input-log.jsonl`）。
    pub fn input_log_path() -> Option<PathBuf> {
        paths::user_data_dir().map(|dir| dir.join("input-log.jsonl"))
    }

    /// 按開關接上 / 拆掉輸入日誌。
    pub(super) fn open_input_log(&mut self, enabled: bool) {
        match Self::input_log_path().filter(|_| enabled) {
            Some(path) => {
                tracing::info!(path = %path.display(), "输入日志开着");
                self.engine.set_input_logger(Box::new(InputLog::open(path)));
                let version = self.version.clone();
                self.engine.log_session(&version, "macos");
            }
            None => self.engine.set_input_logger(Box::new(NoInputLogger)),
        }
    }

    pub(super) fn clear_input_log(&mut self) {
        let Some(path) = Self::input_log_path() else {
            return;
        };
        self.engine.set_input_logger(Box::new(NoInputLogger));
        match InputLog::clear(&path) {
            Ok(()) => {
                tracing::info!(path = %path.display(), "输入日志已清空");
                self.preferences.set_status("输入日志已清空");
            }
            Err(error) => {
                tracing::warn!(%error, "输入日志清空失败");
                self.preferences.set_status("输入日志清空失败，见日志");
            }
        }
        let enabled = self.input_log_enabled.unwrap_or(false);
        self.open_input_log(enabled);
    }
}

/// 配置原文裡的金鑰行抹掉值（`api_key = "…"`；`api_key_env` 是變數名，不抹）。
pub(super) fn redact_secrets(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("api_key") && !trimmed.starts_with("api_key_env") {
                "api_key = \"<已隐去>\"".to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 寫進系統剪貼簿。
pub(super) fn copy_to_pasteboard(text: &str) {
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    pasteboard.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
}

pub(super) fn open_with_system(args: &[&str]) {
    if let Err(error) = std::process::Command::new("open").args(args).spawn() {
        tracing::warn!(%error, ?args, "open 失败");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn redaction_hides_key_values_only() {
        let text =
            "[predict]\napi_key = \"sk-secret\"\napi_key_env = \"QINGJIAN_API_KEY\"\nmodel = \"x\"";
        let redacted = super::redact_secrets(text);
        assert!(!redacted.contains("sk-secret"));
        assert!(redacted.contains("api_key_env = \"QINGJIAN_API_KEY\""));
        assert!(redacted.contains("model = \"x\""));
    }
}
