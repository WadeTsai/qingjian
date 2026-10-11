//! 配置熱載入：config.toml 改了就整份重新套用到 Engine 與視窗；啟用期間的定時事務。

mod text_replacements;
mod watch;

pub(super) use text_replacements::TextReplacement;
pub(super) use watch::ConfigWatch;

use qingjian_render::TextSizes;

use super::init::load_glossary;
use super::*;

impl Host {
    /// 把當前配置推給 Engine 與介面：模糊音 / 模式鍵 / 翻頁 / 外觀直接設；學習語言變了換釋義表；
    /// `[predict]` 變了（或 `force`）才重建 Predictor；最後重新整理雲朵標識、選單勾選與設定視窗。
    pub fn apply_config(&mut self, force: bool) {
        let config = self.settings.config().clone();
        self.engine.set_fuzzy(config.fuzzy);
        self.engine.set_traditional_mode(config.general.traditional);
        self.engine
            .set_full_width_punctuation(config.general.full_width_punctuation);
        self.apply_custom_phrases(&config);
        self.engine.set_mode_keys(config.shortcut.mode);
        self.engine.set_chinese_first(config.general.chinese_first);
        self.engine.set_emoji_candidates(config.general.emoji);
        self.engine
            .set_english_in_chinese(config.general.english_in_chinese);
        self.engine
            .set_shift_letter_compose(config.general.shift_letter.compose());
        self.apply_scheme(config.general.scheme(), config.general.wubi());
        self.engine
            .set_shuangpin_raw_preedit(config.general.shuangpin_raw_preedit);
        self.engine
            .set_zhuyin_layout(config.general.zhuyin_layout());
        self.engine.set_learning(config.general.learning);
        logging::set_level(config.general.log_level);
        self.translation_keys = config.shortcut.translation_keys();
        self.delete_keys = config.shortcut.delete_keys();
        self.translate_keys = config.shortcut.translate_selection;
        self.page_size = config.general.page_size();
        self.cloud_slots = config.predict.slots;
        self.page_keys = config.general.page_keys();
        self.punct_commits = config.general.punct_commits;
        self.preedit_mode = config.general.preedit;
        self.english_candidates = config.general.english_candidates;
        self.apps = config.apps.clone();
        self.window.set_appearance(config.general.appearance());
        self.window.set_layout(config.general.layout);
        if self.layout != config.general.layout
            || self.horizontal_grid != config.general.horizontal_grid
        {
            self.layout = config.general.layout;
            self.horizontal_grid = config.general.horizontal_grid;
            self.session.collapse();
        }
        self.window.set_font(&config.general.font);
        self.themes.refresh();
        self.window
            .set_render_theme(self.render_theme(&config.general));
        self.window.set_renderer(config.general.renderer);
        self.apply_learning_language(&config.general);
        if self.input_log_enabled != Some(config.general.input_log) {
            self.input_log_enabled = Some(config.general.input_log);
            self.open_input_log(config.general.input_log);
        }
        if force || config.predict != self.applied_predict {
            if config.predict.enabled {
                // 沒金鑰等失敗只記日誌、退回不聯想：輸入優先於一切附加功能
                match CloudPredictor::new(&config.predict) {
                    Ok(predictor) => self.engine.set_predictor(Box::new(predictor)),
                    Err(error) => {
                        tracing::warn!(%error, "云联想未启用");
                        self.engine.set_predictor(Box::new(NoPredictor));
                    }
                }
                // 釋義兜底隨雲聯想一起開：釋義表裡沒有的詞上屏後問雲端寫進個人釋義表
                match CloudGlossFiller::new(&config.predict) {
                    Ok(filler) => self.engine.set_gloss_filler(Box::new(filler)),
                    Err(error) => {
                        tracing::warn!(%error, "释义兜底未启用");
                        self.engine.set_gloss_filler(Box::new(NoGlossFiller));
                    }
                }
            } else {
                self.engine.set_predictor(Box::new(NoPredictor));
                self.engine.set_gloss_filler(Box::new(NoGlossFiller));
            }
            self.monitor.stop();
            self.sentence = None;
            self.applied_predict = config.predict.clone();
        }
        if force || config.dictionaries != self.applied_dictionaries {
            self.reload_dictionaries();
        }
        if self.applied_model.as_ref() != Some(&config.model) {
            if config.model.enabled {
                self.load_local_model();
            } else {
                self.unload_local_model();
            }
            self.applied_model = Some(config.model.clone());
        }
        let cloud_active = self.engine.prediction_enabled();
        self.indicator.set_cloud(cloud_active);
        self.indicator.update();
        self.menu.sync(&config, cloud_active, self.settings.error());
        let key_present = config
            .predict
            .api_key
            .as_deref()
            .is_some_and(|key| !key.trim().is_empty())
            || std::env::var(&config.predict.api_key_env).is_ok_and(|key| !key.trim().is_empty());
        self.dictionary_list = self.dictionary_infos();
        self.preferences.sync(
            &config,
            key_present,
            self.settings.error(),
            &self.dictionary_list,
            &self.update_status,
        );
        self.sync_update();
    }

    /// 配置裡的自定義短語，`[general] system_text_replacements` 開著時再並上系統的文本替換，一起推給 Engine。
    fn apply_custom_phrases(&mut self, config: &qingjian_platform::Config) {
        let phrases = if config.general.system_text_replacements {
            qingjian_core::custom_phrase::merge_replacements(
                &config.custom_phrases,
                self.text_replacements
                    .iter()
                    .map(|(code, text)| (code.as_str(), text.as_str())),
            )
        } else {
            config.custom_phrases.clone()
        };
        if let Err(error) = self.engine.set_custom_phrases(phrases) {
            tracing::warn!(%error, "自定义短语配置未应用");
        }
    }

    /// 重讀系統的文本替換（啟用輸入法時調，系統設定裡改過就能跟上）；列表變了才重新套用短語。
    pub fn refresh_text_replacements(&mut self) {
        let latest = text_replacements::read_system();
        if latest == self.text_replacements {
            return;
        }
        tracing::info!(count = latest.len(), "系统文本替换已读取");
        self.text_replacements = latest;
        let config = self.settings.config().clone();
        if config.general.system_text_replacements {
            self.apply_custom_phrases(&config);
        }
    }

    /// 學習語言變了就換釋義表，`off` 換成不翻譯；檔案缺失或壞了保持原樣，只記日誌。
    pub(super) fn apply_learning_language(&mut self, general: &GeneralConfig) {
        if general.learning_language_off() {
            if self.learning_language.take().is_some() {
                self.engine.set_translator(Box::new(NoTranslator));
                tracing::info!("学习语言已关，不显示译文");
            }
            return;
        }
        let code = general.learning_language.as_str();
        let Ok(language) = code.parse::<Language>() else {
            tracing::warn!(code, "不认识的学习语言，保持不变");
            return;
        };
        if self.learning_language == Some(language) {
            return;
        }
        match load_glossary(language) {
            Ok(glossary) => {
                tracing::info!(
                    language = language.code(),
                    glosses = glossary.len(),
                    "释义表已切换"
                );
                self.engine.set_translator(Box::new(glossary));
                self.learning_language = Some(language);
            }
            Err(error) => tracing::warn!(%error, "释义表加载失败，学习语言不变"),
        }
    }

    /// 配置檔案被手改過就熱載入；啟用輸入法時和監視定時器都會調。
    pub fn reload_config_if_changed(&mut self) {
        if self.settings.reload_if_changed() {
            self.apply_config(false);
        }
    }

    /// 啟用期間的定時器每秒調一次：看配置檔案，再看學習資料要不要落盤。
    /// 學習資料原本只在停用輸入法時儲存，程序被 launchd 殺掉就丟一整段；現在最多丟 [`LEARNING_FLUSH_INTERVAL`] 這麼久。
    /// 配置裡選的主題，蓋上設定裡的字號與動畫開關。
    fn render_theme(&self, general: &GeneralConfig) -> qingjian_render::Theme {
        let sizes = TextSizes {
            candidate: general.candidate_font_size.get(),
            annotation: general.annotation_font_size.get(),
        };
        self.themes
            .resolve(general.theme_id(), false)
            .with_text_sizes(sizes)
            .with_animations(general.animations)
    }

    /// 沒有新資料時 flush 是空操作（各表按 dirty 位判斷），不會每分鐘碰一次磁碟。
    pub fn tick(&mut self) {
        self.reload_config_if_changed();
        // 使用者主題目錄裡的 theme.json 改了就重讀，當前主題當場換上（邊改主題邊看效果）
        if self.themes.refresh() {
            let general = self.settings.config().general.clone();
            tracing::info!(id = general.theme_id(), "主题目录有变化，已重读");
            self.window.set_render_theme(self.render_theme(&general));
        }
        let learned = self.engine.poll_glosses();
        if learned > 0 {
            tracing::info!(learned, "释义兜底写入个人释义表");
        }
        if self.last_flush.elapsed() >= LEARNING_FLUSH_INTERVAL {
            self.engine.flush_learning();
            self.last_flush = std::time::Instant::now();
        }
        if let Some(updates) = &self.updates {
            updates.poll(&self.settings.config().update);
        }
        self.sync_update();
    }

    /// 檢查更新的狀態變了就刷選單裡的「有新版本」與「關於」頁；沒變什麼都不做。
    pub fn sync_update(&mut self) {
        let Some(updates) = &self.updates else {
            return;
        };
        let config = self.settings.config();
        let status = UpdateStatus {
            available: updates.available(&config.update).map(|found| found.version),
            checking: updates.checking(),
            checked: updates.checked_at() > 0,
            dev_build: updates.is_dev_build(),
        };
        if status != self.update_status {
            self.menu.sync_update(status.available.as_deref());
            self.preferences.sync_update(config, &status);
            self.update_status = status;
        }
    }

    /// 按配置的兩條軸裝配引擎：拼音側（全拼 / 雙拼 / 注音 / 關）與形碼側（五筆）。
    /// 兩邊都開就是混輸——編碼打全的形碼候選在前，見 `Engine::query_mixed`。
    ///
    /// 開了形碼卻找不到碼錶時只警告並退回只用拼音——配置說五筆、引擎一個字都打不出更糟。
    fn apply_scheme(&mut self, pinyin: Scheme, wubi: bool) {
        self.engine.set_shuangpin(pinyin.shuangpin());
        self.engine.set_zhuyin_mode(pinyin == Scheme::Zhuyin);
        // 拼音側關掉且形碼開著才是「只用形碼」；兩邊都關著時留拼音兜底（否則一個候選都沒有）
        self.engine.set_phonetic(pinyin.is_on() || !wubi);
        if !wubi {
            self.engine.set_code_table(None);
            return;
        }
        let Some(path) = paths::code_table_path() else {
            tracing::warn!("选了形码方案但找不到码表，仍按拼音输入；随包数据里应当带一份");
            self.engine.set_code_table(None);
            return;
        };
        match qingjian_dictionary::CodeTable::from_path(&path) {
            Ok(table) => {
                tracing::info!(table = %path.display(), entries = table.len(), "形码码表已载入");
                self.engine.set_code_table(Some(table));
            }
            Err(error) => {
                tracing::error!(%error, table = %path.display(), "形码码表读不了，仍按拼音输入");
                self.engine.set_code_table(None);
            }
        }
    }
}
