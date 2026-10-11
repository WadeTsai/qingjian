//! 配置檔案的執行時狀態：路徑、當前生效的值、上次讀取時的修改時間。
//!
//! 啟動、選單開關、切回輸入法時的熱載入都從這裡拿 `Config`，殼只認這一份。
//! 解析失敗不讓輸入法退出（輸入優先於一切附加功能）：保留上一份能用的配置，把錯誤留給選單顯示。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use qingjian_platform::Config;

use super::paths;

pub struct Settings {
    /// 配置檔案路徑；拿不到使用者目錄時為 `None`，此時一切按預設值、選單開關不落盤。
    path: Option<PathBuf>,

    /// 當前生效的配置。
    config: Config,

    /// 上次成功讀取時檔案的修改時間，用來判斷使用者有沒有手改過。
    modified: Option<SystemTime>,

    /// 最近一次讀取失敗的原因；成功後清掉。
    error: Option<String>,
}

impl Settings {
    /// 首次載入：先讀同目錄的 `.env`，檔案不存在就寫模板，再讀配置。
    pub fn load() -> Self {
        let mut settings = Self {
            path: paths::config_file(),
            config: Config::default(),
            modified: None,
            error: None,
        };
        if let Some(path) = settings.path.clone() {
            load_dotenv(&path);
            match Config::write_template_if_missing(&path) {
                Ok(true) => tracing::info!(path = %path.display(), "已写出配置模板"),
                Ok(false) => {}
                Err(error) => tracing::warn!(%error, "写配置模板失败"),
            }
            settings.read(&path);
        }
        settings
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// 最近一次讀取的錯誤文案，給選單顯示。
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// 檔案的修改時間和上次讀的不一樣就重讀；返回是否重讀了（重讀失敗也算，配置沒變但錯誤狀態變了）。
    pub fn reload_if_changed(&mut self) -> bool {
        let Some(path) = self.path.clone() else {
            return false;
        };
        if modified_time(&path) == self.modified {
            return false;
        }
        self.reload();
        true
    }

    /// 強制重讀，`.env` 也再讀一遍（不覆蓋已有的環境變數，只補新加的）。
    pub fn reload(&mut self) {
        if let Some(path) = self.path.clone() {
            load_dotenv(&path);
            self.read(&path);
        }
    }

    /// 原地改一個布林鍵並重讀，見 [`Self::set_value`]。
    pub fn set_bool(&mut self, section: &str, key: &str, value: bool) -> bool {
        self.set_value(section, key, value)
    }

    /// 原地改一個鍵並重讀。寫失敗只記日誌，返回 `false`。
    pub fn set_value(
        &mut self,
        section: &str,
        key: &str,
        value: impl Into<toml_edit::Value>,
    ) -> bool {
        let Some(path) = self.path.clone() else {
            tracing::warn!("没有配置文件路径，设置不落盘");
            return false;
        };
        let value = value.into();
        if let Err(error) = Config::set_value(&path, section, key, value.clone()) {
            tracing::warn!(%error, section, key, "写配置失败");
            return false;
        }
        tracing::info!(section, key, value = %value.to_string().trim(), "配置已改");
        self.read(&path);
        true
    }

    /// 把一個環境變數（金鑰）寫進配置同目錄的 `.env` 並立即注入當前程序。
    /// 檔案僅本使用者可讀；值不進日誌。返回是否成功。
    pub fn set_env_var(&self, name: &str, value: &str) -> bool {
        let Some(path) = &self.path else {
            tracing::warn!("没有配置目录，密钥无处可存");
            return false;
        };
        // 先登記再動檔案：後面哪一步失敗，日誌裡都不會出現這個值
        qingjian_platform::logs::secrets::register(value);
        let env_file = path.with_file_name(".env");
        let existing = std::fs::read_to_string(&env_file).unwrap_or_default();
        let prefix = format!("{name}=");
        let mut lines: Vec<&str> = existing
            .lines()
            .filter(|line| !line.trim_start().starts_with(&prefix))
            .collect();
        let entry = format!("{prefix}{value}");
        lines.push(&entry);
        let content = format!("{}\n", lines.join("\n"));
        // 原子寫且僅本使用者可讀（0600）
        let written = qingjian_core::storage::write_atomic_private(&env_file, |file| {
            file.write_all(content.as_bytes())
        });
        if let Err(error) = written {
            tracing::warn!(%error, path = %env_file.display(), "写 .env 失败");
            return false;
        }
        // 解析錯誤的原文裡帶著出錯的那一行（含金鑰），不進日誌
        if dotenvy::from_path_override(&env_file).is_err() {
            tracing::warn!(path = %env_file.display(), ".env 写入后重新加载失败");
            return false;
        }
        tracing::info!(name, path = %env_file.display(), "密钥已写入 .env");
        true
    }

    fn read(&mut self, path: &Path) {
        match Config::load(path) {
            Ok(config) => {
                tracing::info!(
                    path = %path.display(),
                    predict = config.predict.enabled,
                    fuzzy = config.fuzzy.any(),
                    "配置已加载"
                );
                self.config = config;
                self.error = None;
            }
            Err(error) => {
                tracing::warn!(%error, "配置读取失败，沿用上一份");
                self.error = Some(error.to_string());
            }
        }
        // 失敗也記時間：同一份壞檔案不用每次啟用都重讀一遍
        self.modified = modified_time(path);
    }
}

/// 輸入法程序由 launchd 拉起，看不到 shell 的環境變數：配置同目錄的 `.env`（如 `QINGJIAN_API_KEY=...`）先讀進環境。
fn load_dotenv(config_path: &Path) {
    let env_file = config_path.with_file_name(".env");
    match dotenvy::from_path(&env_file) {
        Ok(()) => tracing::info!(path = %env_file.display(), "已加载 .env"),
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        // 解析錯誤的原文裡帶著出錯的那一行（含金鑰），只記是哪一類錯
        Err(dotenvy::Error::LineParse(_, position)) => {
            tracing::warn!(path = %env_file.display(), position, ".env 有一行格式不对，整个文件没读进来");
        }
        Err(dotenvy::Error::Io(error)) => {
            tracing::warn!(path = %env_file.display(), %error, ".env 读取失败");
        }
        Err(_) => tracing::warn!(path = %env_file.display(), ".env 读取失败"),
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}
