//! 詞庫管理：隨包領域詞庫開關，使用者匯入詞庫的匯入 / 移除 / 開關，重新載入。

mod info;

pub use info::DictionaryInfo;

use super::init::{dictionary_info, string_array, toggle_membership};
use super::*;

impl Host {
    /// 「詞庫」頁的列表：隨包領域詞庫在前、使用者目錄 `dicts/` 裡的在後（含關掉的）。
    pub(super) fn dictionary_infos(&self) -> Vec<DictionaryInfo> {
        let config = &self.settings.config().dictionaries;
        let mut infos = Vec::new();
        if let Some(dir) = paths::bundled_dicts_dir() {
            for (stem, path) in extra_dictionaries::list(&dir) {
                let enabled = config.is_domain_enabled(&stem);
                infos.push(dictionary_info(stem, path, true, enabled));
            }
        }
        if let Some(dir) = paths::dicts_dir() {
            for (stem, path) in extra_dictionaries::list(&dir) {
                let enabled = config.is_enabled(&stem);
                infos.push(dictionary_info(stem, path, false, enabled));
            }
        }
        infos
    }

    /// 匯入一本詞庫（TSV / Rime yaml / .qj）到 `dicts/`，然後重新裝配。
    pub fn import_dictionary(&mut self, source: &std::path::Path) {
        let Some(dir) = paths::dicts_dir() else {
            return;
        };
        match qingjian_dictionary::import::import(source, &dir) {
            Ok(imported) => {
                tracing::info!(name = %imported.name, entries = imported.entries, path = %imported.path.display(), "词库已导入");
                // 之前關掉過同名詞庫的，匯入後自動開啟
                let stem = imported
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_owned();
                self.set_dictionary_enabled(&stem, true);
                self.reload_dictionaries();
                self.apply_config(false);
            }
            Err(error) => tracing::warn!(source = %source.display(), %error, "词库导入失败"),
        }
    }

    /// 移除一本匯入的詞庫：檔案挪到 `dicts/removed/`（可手工找回），配置裡的開關項一併清掉。隨包詞庫不能移除。
    pub fn remove_dictionary(&mut self, index: usize) {
        let Some(info) = self.dictionary_list.get(index).cloned() else {
            return;
        };
        if info.builtin {
            return;
        }
        let Some(dir) = paths::dicts_dir() else {
            return;
        };
        let removed = dir.join("removed");
        let moved = std::fs::create_dir_all(&removed).and_then(|()| {
            std::fs::rename(
                &info.path,
                removed.join(info.path.file_name().unwrap_or_default()),
            )
        });
        match moved {
            Ok(()) => tracing::info!(name = %info.name, "词库已移除（挪到 dicts/removed/）"),
            Err(error) => tracing::warn!(%error, path = %info.path.display(), "移除词库失败"),
        }
        self.set_dictionary_enabled(&info.stem, true);
        self.reload_dictionaries();
        self.apply_config(false);
    }

    /// 改 `[dictionaries] disabled`：`on` 為真從列表裡去掉，否則加進去。
    pub fn set_dictionary_enabled(&mut self, stem: &str, on: bool) {
        let disabled = self.settings.config().dictionaries.disabled.clone();
        if let Some(disabled) = toggle_membership(disabled, stem, !on) {
            self.settings.set_value(
                "dictionaries",
                "disabled",
                toml_edit::Value::Array(string_array(&disabled)),
            );
        }
    }

    /// 改 `[dictionaries] domains`（隨包領域詞庫）：`on` 為真加進列表，否則去掉。
    pub fn set_domain_enabled(&mut self, stem: &str, on: bool) {
        let domains = self.settings.config().dictionaries.domains.clone();
        if let Some(domains) = toggle_membership(domains, stem, on) {
            self.settings.set_value(
                "dictionaries",
                "domains",
                toml_edit::Value::Array(string_array(&domains)),
            );
        }
    }

    /// 按當前配置重新載入 `dicts/` 目錄（匯入、移除、開關之後）。
    pub fn reload_dictionaries(&mut self) {
        let config = self.settings.config().dictionaries.clone();
        let loaded = extra_dictionaries::load(
            paths::bundled_dicts_dir().as_deref(),
            paths::dicts_dir().as_deref(),
            &config,
        );
        tracing::info!(count = loaded.len(), "附加词库已装配");
        self.engine.set_extra_dictionaries(loaded);
        self.applied_dictionaries = config;
    }
}
