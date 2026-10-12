//! 配置熱載入：空閒時看 `config.toml` 的 mtime，改了就重讀並應用（與 macOS 殼對齊）。
//! 便宜的設定無條件重設；雲聯想 / 釋義表按配置變化重建，附加詞庫也檢查檔案增刪與更新。熱載入狀態在 [`ConfigReload`]。

mod state;

#[cfg(test)]
mod tests;

use qingjian_render::ThemeLibrary;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use qingjian_core::{Engine, Language, NoGlossFiller, NoPredictor, NoTranslator};
use qingjian_platform::{Config, code_tables};
use qingjian_predict::{CloudGlossFiller, CloudPredictor, PredictConfig};

pub(super) use self::state::ConfigReload;
pub use self::state::DataDirs;

/// 看配置檔案 mtime 的最短間隔；工人迴圈空閒時按它等，重排的短節拍來得更勤時按這個節流。
pub(super) const CONFIG_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// 檢查更新的結果檔名，在使用者資料目錄下（見 `qingjian-update::UpdateState`）。
const UPDATE_STATE_FILE: &str = "update.json";
use super::{Router, RouterConfig};
use crate::assembly;

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// 按 `[predict]` 接雲聯想與釋義兜底；關著或缺金鑰就退回本地實現。啟動與熱載入共用。
pub fn attach_cloud(engine: &mut Engine, predict: &PredictConfig) {
    if !predict.enabled {
        tracing::info!("云联想未开启（[predict] enabled = false）");
        engine.set_predictor(Box::new(NoPredictor));
        engine.set_gloss_filler(Box::new(NoGlossFiller));
        return;
    }
    match CloudPredictor::new(predict) {
        Ok(predictor) => {
            engine.set_predictor(Box::new(predictor));
            tracing::info!(model = %predict.model, "云联想已接入");
        }
        Err(error) => {
            tracing::warn!(%error, "云联想接入失败（缺 API key？），退回本地候选");
            engine.set_predictor(Box::new(NoPredictor));
        }
    }
    match CloudGlossFiller::new(predict) {
        Ok(filler) => engine.set_gloss_filler(Box::new(filler)),
        Err(error) => {
            tracing::warn!(%error, "释义兜底未启用");
            engine.set_gloss_filler(Box::new(NoGlossFiller));
        }
    }
}

/// 學習語言變了就換釋義表：關是不翻譯；換語言重灌隨包 + 個人釋義表，沒有這門語言的表或裝不上就保持原樣。
/// 換成功（或關掉）返回 true。
fn swap_translator(
    engine: &mut Engine,
    language: Option<Language>,
    root: &Path,
    user_dir: Option<&Path>,
) -> bool {
    let Some(language) = language else {
        engine.set_translator(Box::new(NoTranslator));
        tracing::info!("学习语言已关，不显示译文");
        return true;
    };
    let Some(path) = assembly::glossary_file(root, language) else {
        tracing::warn!(
            language = language.code(),
            "没有这门语言的释义表，学习语言不变"
        );
        return false;
    };
    match assembly::load_glossary(language, &path, user_dir) {
        Ok(glossary) => {
            tracing::info!(language = language.code(), "释义表已切换");
            engine.set_translator(Box::new(glossary));
            true
        }
        Err(error) => {
            tracing::warn!(%error, "释义表加载失败，学习语言不变");
            false
        }
    }
}

impl Router {
    /// 檢查更新查到了要提示的新版本（開關關著、本地開發包都不算）。
    pub(super) fn update_available(&self) -> bool {
        self.reload.as_ref().is_some_and(|reload| {
            reload
                .updates
                .as_ref()
                .is_some_and(|updates| updates.available(&reload.update).is_some())
        })
    }

    /// `config.toml` 路徑；沒開熱載入（測試）時為 `None`。
    pub(super) fn config_path(&self) -> Option<&Path> {
        self.reload
            .as_ref()
            .map(|reload| reload.config_path.as_path())
    }

    /// 開啟熱載入：記下路徑與當前已應用的 predict / dictionaries / aux_code / 學習語言，
    /// 以及啟動用的那批資料目錄。目錄必須與啟動同款語義（`dicts/` / `codes/`），
    /// 熱載入才找得到檔案。
    pub fn watch_config(
        &mut self,
        config: &Config,
        config_path: PathBuf,
        root: PathBuf,
        dirs: DataDirs,
    ) {
        let last_mtime = mtime(&config_path);
        let code_files = dirs.code_snapshot();
        let dictionary_files = dirs.dict_snapshot();
        let updates = dirs.user_root.as_deref().map(|dir| {
            qingjian_update::Checker::new(dir.join(UPDATE_STATE_FILE), env!("CARGO_PKG_VERSION"))
        });
        self.reload = Some(ConfigReload {
            config_path,
            last_check: Instant::now(),
            root,
            dirs,
            code_files,
            last_mtime,
            applied_predict: config.predict.clone(),
            applied_dictionaries: config.dictionaries.clone(),
            applied_aux_code: config.aux_code.clone(),
            dictionary_files,
            applied_language: assembly::learning_language(config),
            update: config.update.clone(),
            updates,
        });
    }

    /// 空閒時調；一秒內只真正看一次。配置檔案或使用者 `codes/` 下的檔案變了就重灌；
    /// 解析失敗保持原配置，mtime 照記（不每秒重試同一個壞檔案）。
    pub fn poll_config_reload(&mut self) {
        let Some(reload) = &mut self.reload else {
            return;
        };
        if reload.last_check.elapsed() < CONFIG_POLL_INTERVAL {
            return;
        }
        reload.last_check = Instant::now();
        if let Some(updates) = &reload.updates {
            updates.poll(&reload.update);
        }
        // 使用者 `dicts/` 目錄檔案增刪或更新：與配置改動無關，下一拍就生效
        let files = reload.dirs.dict_snapshot();
        if files != reload.dictionary_files {
            // 配置損壞也繼續使用上次有效的詞庫開關；檔案變化不觸發配置重試。
            self.engine
                .set_extra_dictionaries(reload.load_dictionaries());
            reload.dictionary_files = files;
        }
        // 使用者主題檔案改了：戳變了設定就不等，UI 執行緒重讀主題當場換上（邊改主題邊看效果）
        if let Some(dir) = qingjian_platform::dirs::themes_dir() {
            let stamp = ThemeLibrary::stamp(&dir);
            if stamp != self.config.themes_stamp {
                self.config.themes_stamp = stamp;
                self.candidates.configure(self.config.render_settings());
            }
        }
        let Some(reload) = &mut self.reload else {
            return;
        };
        let config_changed = {
            let current = mtime(&reload.config_path);
            let changed = current != reload.last_mtime;
            reload.last_mtime = current;
            changed
        };
        // 使用者 `codes/` 下的檔案增刪或更新（設定頁剛匯入 / 移除一張碼錶）：不必等配置改動，下一拍就生效
        let codes_changed = {
            let current = reload.dirs.code_snapshot();
            let changed = current != reload.code_files;
            reload.code_files = current;
            changed
        };
        let path = reload.config_path.clone();
        if codes_changed {
            self.reload_aux_codes();
        }
        if !config_changed {
            return;
        }
        match Config::load(&path) {
            Ok(mut config) => {
                // 與啟動時一樣：Windows 版不管檔案裡寫什麼都出繁體
                super::config::enforce_traditional(&mut config);
                self.apply_config(&config);
                tracing::info!("配置已热加载");
            }
            Err(error) => tracing::error!(%error, "配置热加载解析失败，保持原配置"),
        }
    }

    /// 應用新配置。學習語言變了換釋義表（詞彙等級表啟動時已全裝，不用換）。
    fn apply_config(&mut self, config: &Config) {
        self.engine.set_fuzzy(config.fuzzy);
        // Windows 端此前漏裝 custom_phrases（macOS 端 apply_config 有）；失敗只記日誌，保持原短語
        if let Err(error) = self
            .engine
            .set_custom_phrases(config.custom_phrases.clone())
        {
            tracing::warn!(%error, "自定义短语配置未应用，保持原短语");
        }
        self.engine.set_learning(config.general.learning);
        self.engine.set_mode_keys(config.shortcut.mode);
        self.engine
            .set_aux_code_key(config.general.aux_code_key(), config.general.page_keys());
        self.engine
            .set_aux_keep_empty(config.general.aux_code_keep_empty);
        self.engine.set_aux_enabled(config.aux_code.enabled);
        self.engine.set_aux_show(config.general.aux_code_show);
        self.engine.set_chinese_first(config.general.chinese_first);
        self.engine.set_emoji_candidates(config.general.emoji);
        self.engine
            .set_english_in_chinese(config.general.english_in_chinese);
        self.engine
            .set_shift_letter_compose(config.general.shift_letter.compose());
        self.engine
            .set_shuangpin_raw_preedit(config.general.shuangpin_raw_preedit);
        self.engine
            .set_zhuyin_layout(config.general.zhuyin_layout());
        let previous = self.config.render_settings();
        let themes_stamp = self.config.themes_stamp;
        self.config = RouterConfig::from(config);
        self.config.themes_stamp = themes_stamp;
        // 繁體輸出與拼音側方案要等新配置就位再算：它們還看聚焦會話的語言 profile（見 session 的 traditional_output / scheme）。
        // 拼音側與形碼側一起裝配（雙拼 / 注音 / 混輸都在裡面）
        self.engine.set_traditional_mode(self.traditional_output());
        self.reload_code_table(self.scheme(), self.config.wubi);
        let settings = self.config.render_settings();
        if settings != previous {
            self.candidates.configure(settings);
        }
        self.reconcile_status();
        self.apply_model_config(&config.model);

        let Some(reload) = &mut self.reload else {
            return;
        };
        reload.update = config.update.clone();
        if config.predict != reload.applied_predict {
            attach_cloud(&mut self.engine, &config.predict);
            reload.applied_predict = config.predict.clone();
        }
        let language = assembly::learning_language(config);
        if language != reload.applied_language
            && swap_translator(
                &mut self.engine,
                language,
                &reload.root,
                reload.dirs.user_root.as_deref(),
            )
        {
            reload.applied_language = language;
        }
        if config.dictionaries != reload.applied_dictionaries {
            reload.applied_dictionaries = config.dictionaries.clone();
            self.engine
                .set_extra_dictionaries(reload.load_dictionaries());
        }
        if config.aux_code != reload.applied_aux_code {
            reload.applied_aux_code = config.aux_code.clone();
            self.reload_aux_codes();
        }
    }

    /// 按當前配置重灌輔碼碼錶：`codes/` 目錄變了或 `[aux_code]` 變了都走這裡。
    fn reload_aux_codes(&mut self) {
        let Some(reload) = &self.reload else {
            return;
        };
        let tables = code_tables::load(
            reload.dirs.bundled_codes.as_deref(),
            reload.dirs.user_codes.as_deref(),
            &reload.applied_aux_code,
        );
        if let Some(reload) = &mut self.reload {
            reload.code_files = reload.dirs.code_snapshot();
        }
        tracing::info!(count = tables.len(), "辅码码表已重装");
        self.engine.set_aux_codes(tables);
    }
}
