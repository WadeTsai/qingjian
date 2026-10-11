//! 選單與偏好設定視窗的動作：只改 config.toml（或觸發一次性操作），改完由 apply_config 統一生效。

use super::diagnostics::{copy_to_pasteboard, open_with_system};
use super::*;
use crate::preferences::DEFAULT_FONT_LABEL;
use qingjian_platform::{FontSize, ShiftLetter};

impl Host {
    /// 寫短語前讀取檔案；外部規則有變化時同步列表並請使用者重新確認。
    fn phrases_are_current(&mut self) -> bool {
        let Some(path) = self.settings.path() else {
            return false;
        };
        match qingjian_platform::Config::load(path) {
            Ok(latest) if latest.custom_phrases == self.settings.config().custom_phrases => true,
            Ok(_) => {
                self.settings.reload();
                self.apply_config(false);
                let message = "规则已在其他地方修改，请重新确认后操作。";
                self.preferences.set_phrase_error(message);
                self.preferences.set_status(message);
                false
            }
            Err(error) => {
                self.preferences.set_phrase_error(&error.to_string());
                self.preferences.set_status(&error.to_string());
                false
            }
        }
    }

    /// 表格中的啟用開關只修改所選規則。
    pub fn set_phrase_enabled(&mut self, index: usize, enabled: bool) {
        if !self.phrases_are_current() {
            return;
        }
        let mut phrases = self.settings.config().custom_phrases.clone();
        let Some(phrase) = phrases.get_mut(index) else {
            return;
        };
        phrase.enabled = enabled;
        let Some(path) = self.settings.path() else {
            return;
        };
        let result = qingjian_platform::Config::set_custom_phrases(path, &phrases);
        self.settings.reload();
        self.apply_config(false);
        if let Err(error) = result {
            self.preferences.set_status(&error);
        }
    }

    /// 選單動作。開關類先落盤再熱載入，選單勾選狀態永遠來自檔案裡的值。
    pub fn perform(&mut self, action: MenuAction) {
        tracing::info!(?action, "菜单");
        match action {
            MenuAction::ToggleCloud => {
                let on = !self.settings.config().predict.enabled;
                if self.settings.set_bool("predict", "enabled", on) {
                    self.apply_config(false);
                }
            }
            MenuAction::ToggleFuzzy(index) => {
                let name = FuzzyRules::NAMES[index];
                let on = !self.settings.config().fuzzy.is_on(name);
                if self.settings.set_bool("fuzzy", name, on) {
                    self.apply_config(false);
                }
            }
            MenuAction::OpenPreferences => {
                self.preferences.sync_usage(
                    &self.engine.usage_summary(),
                    &self.engine.vocabulary_summary(),
                    self.learning_language,
                );
                self.preferences.show();
            }
            MenuAction::OpenLogs => {
                if let Some(dir) = logging::log_dir() {
                    open_with_system(&[&dir.to_string_lossy()]);
                }
            }
            MenuAction::OpenDownload => open_with_system(&[qingjian_update::DOWNLOAD_URL]),
        }
    }

    /// 設定窗口裡改了一個控制元件：寫配置、熱載入；寫不成（非法組合、空文本）也要把控制元件同步回真實值。
    pub fn change_setting(&mut self, setting: Setting, value: SettingValue) {
        // 金鑰值不進日誌
        tracing::info!(?setting, "设置");
        let config = self.settings.config().clone();
        match (setting, value) {
            (Setting::NewPhrase, _) => {
                self.preferences.edit_phrase(&config, None);
                return;
            }
            (Setting::EditPhrase, _) => {
                if let Some(index) = self.preferences.selected_phrase() {
                    self.preferences.edit_phrase(&config, Some(index));
                }
                return;
            }
            (Setting::CancelPhraseEdit, _) => {
                self.preferences.close_phrase_editor();
                return;
            }

            (Setting::PhraseDraft, _) => return,
            (Setting::SelectPhrase, SettingValue::Index(index)) => {
                self.preferences.select_phrase(&config, index);
                return;
            }
            (Setting::SavePhrase | Setting::DeletePhrase, _) => {
                if !self.phrases_are_current() {
                    return;
                }
                let config = self.settings.config();
                let mut phrases = config.custom_phrases.clone();
                let mut saved_index = phrases.len();
                if setting == Setting::DeletePhrase {
                    let Some(index) = self.preferences.selected_phrase() else {
                        return;
                    };
                    if index >= phrases.len() {
                        return;
                    }
                    phrases.remove(index);
                } else {
                    let (index, draft) = match self.preferences.phrase_draft(config) {
                        Ok(value) => value,
                        Err(error) => {
                            self.preferences.set_phrase_error(&error);
                            self.preferences.set_status(&error);
                            return;
                        }
                    };
                    if let Some(index) = index {
                        let Some(phrase) = phrases.get_mut(index) else {
                            return;
                        };
                        *phrase = draft;
                        saved_index = index;
                    } else {
                        phrases.push(draft);
                    }
                }
                let Some(path) = self.settings.path() else {
                    return;
                };
                if let Err(error) = qingjian_platform::Config::set_custom_phrases(path, &phrases) {
                    self.preferences.set_phrase_error(&error);
                    self.preferences.set_status(&error);
                    return;
                }
                self.preferences.close_phrase_editor();
                self.settings.reload();
                self.apply_config(false);
                if setting == Setting::SavePhrase {
                    self.preferences
                        .select_phrase(self.settings.config(), saved_index + 1);
                }
                self.preferences.set_status("自定义短语已保存");
                return;
            }
            (Setting::FullWidthPunctuation, SettingValue::Index(index)) => {
                self.settings
                    .set_bool("general", "full_width_punctuation", index == 0);
            }
            (Setting::PunctCommits, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "punct_commits", on);
            }
            (Setting::LearningLanguage, SettingValue::Index(index)) => {
                // 選單最後一項是「不顯示譯文」
                let code = self
                    .languages
                    .get(index)
                    .map_or(LEARNING_LANGUAGE_OFF, |language| language.code());
                self.settings
                    .set_value("general", "learning_language", code);
            }
            (Setting::PageSize, SettingValue::Index(index)) => {
                self.settings
                    .set_value("general", "page_size", index as i64 + 1);
            }
            (Setting::PageKeys, SettingValue::Index(index)) => {
                if let Some(pair) = PAGE_KEY_OPTIONS.get(index) {
                    self.settings.set_value("general", "page_keys", *pair);
                }
            }
            (Setting::Appearance, SettingValue::Index(index)) => {
                if let Some(appearance) = Appearance::ALL.get(index) {
                    self.settings
                        .set_value("general", "appearance", appearance.key());
                }
            }
            (Setting::Theme, SettingValue::Index(index)) => {
                // 與偏好設定裡的列表同源：都是主題庫（內建在前、使用者主題按 id 排）
                self.themes.refresh();
                if let Some(theme) = self.themes.themes().get(index) {
                    let id = theme.id().to_owned();
                    self.settings.set_value("general", "theme", &id);
                }
            }
            (Setting::Renderer, SettingValue::Index(index)) => {
                if let Some(renderer) = CandidateRenderer::ALL.get(index) {
                    self.settings
                        .set_value("general", "renderer", renderer.key());
                }
            }
            (Setting::Font, SettingValue::Text(text)) => {
                let font = text.trim();
                let font = if font == DEFAULT_FONT_LABEL { "" } else { font };
                self.settings.set_value("general", "font", font);
            }
            (Setting::Layout, SettingValue::Index(index)) => {
                if let Some(layout) = LayoutMode::ALL.get(index) {
                    self.settings.set_value("general", "layout", layout.key());
                }
            }
            (Setting::CandidateFontSize, SettingValue::Text(text)) => {
                self.set_font_size("candidate", &text, config.general.candidate_font_size);
            }
            (Setting::AnnotationFontSize, SettingValue::Text(text)) => {
                self.set_font_size("annotation", &text, config.general.annotation_font_size);
            }
            (Setting::ResetFontSizes, _) => {
                self.settings.set_value("general", "candidate_font_size", 0);
                self.settings
                    .set_value("general", "annotation_font_size", 0);
            }
            (Setting::Animations, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "animations", on);
            }
            (Setting::HorizontalGrid, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "horizontal_grid", on);
            }
            (Setting::Preedit, SettingValue::Index(index)) => {
                if let Some(mode) = PreeditMode::ALL.get(index) {
                    self.settings.set_value("general", "preedit", mode.key());
                }
            }
            (Setting::QuestionMark, SettingValue::Bool(on)) => {
                self.settings.set_bool("shortcut", "question_mark", on);
            }
            (Setting::ExpressionKey | Setting::QuestionKey, SettingValue::Index(index)) => {
                if let Some(&key) = ModeKeys::CANDIDATES.get(index) {
                    let mut keys = config.shortcut.mode.sanitized();
                    let name = if setting == Setting::ExpressionKey {
                        keys.expression = key;
                        "expression"
                    } else {
                        keys.question = key;
                        "question"
                    };
                    if keys.is_valid() {
                        self.settings.set_value("shortcut", name, key.to_string());
                    } else {
                        tracing::warn!("表达式键与问字键不能相同，未改");
                    }
                }
            }
            (
                Setting::TranslationKeys | Setting::TranslationSecondKeys,
                SettingValue::Text(text),
            ) => match text.parse::<Modifiers>() {
                Ok(chosen) => {
                    let (first, second) = config.shortcut.translation_keys();
                    let (name, other) = if setting == Setting::TranslationKeys {
                        ("translation", second)
                    } else {
                        ("translation_second", first)
                    };
                    if chosen == other {
                        tracing::warn!("两组译词快捷键不能相同，未改");
                    } else {
                        self.settings.set_value("shortcut", name, chosen.key());
                    }
                }
                Err(error) => tracing::warn!(%error, "修饰键组合不合法，未改"),
            },
            (Setting::DeleteCandidateKeys, SettingValue::Text(text)) => {
                match text.parse::<Modifiers>() {
                    Ok(chosen) => {
                        let (first, second) = config.shortcut.translation_keys();
                        if chosen == first || chosen == second {
                            tracing::warn!("删候选的快捷键不能与译词快捷键相同，未改");
                        } else {
                            self.settings
                                .set_value("shortcut", "delete_candidate", chosen.key());
                        }
                    }
                    Err(error) => tracing::warn!(%error, "修饰键组合不合法，未改"),
                }
            }
            (Setting::TranslateSelectionKeys, SettingValue::Text(text)) => {
                match text.parse::<KeyCombo>() {
                    Ok(combo) => {
                        self.settings.set_value(
                            "shortcut",
                            "translate_selection",
                            combo.key_string(),
                        );
                    }
                    Err(error) => tracing::warn!(%error, "快捷键不合法，未改"),
                }
            }
            (Setting::ResetShortcuts, _) => {
                let defaults = ShortcutConfig::default();
                self.settings
                    .set_value("general", "page_keys", PAGE_KEY_OPTIONS[0]);
                self.settings.set_value(
                    "shortcut",
                    "expression",
                    defaults.mode.expression.to_string(),
                );
                self.settings
                    .set_value("shortcut", "question", defaults.mode.question.to_string());
                self.settings
                    .set_bool("shortcut", "question_mark", defaults.mode.question_mark);
                self.settings
                    .set_value("shortcut", "translation", defaults.translation.key());
                self.settings.set_value(
                    "shortcut",
                    "translation_second",
                    defaults.translation_second.key(),
                );
                self.settings.set_value(
                    "shortcut",
                    "translate_selection",
                    defaults.translate_selection.key_string(),
                );
                self.settings.set_value(
                    "shortcut",
                    "delete_candidate",
                    defaults.delete_candidate.key(),
                );
            }
            (Setting::DictionaryEnabled(index), SettingValue::Bool(on)) => {
                if let Some(info) = self.dictionary_list.get(index).cloned() {
                    if info.builtin {
                        self.set_domain_enabled(&info.stem, on);
                    } else {
                        self.set_dictionary_enabled(&info.stem, on);
                    }
                }
            }
            (Setting::DictionaryRemove(index), _) => {
                self.remove_dictionary(index);
                return;
            }
            (Setting::ImportDictionary, _) => {
                if let Some(path) = crate::preferences::choose_dictionary_file() {
                    self.import_dictionary(&path);
                }
                return;
            }
            (Setting::Fuzzy(index), SettingValue::Bool(on)) => {
                self.settings
                    .set_bool("fuzzy", FuzzyRules::NAMES[index], on);
            }
            (Setting::CloudEnabled, SettingValue::Bool(on)) => {
                self.settings.set_bool("predict", "enabled", on);
            }
            (Setting::LocalModelEnabled, SettingValue::Bool(on)) => {
                self.settings.set_bool("model", "enabled", on);
            }
            (Setting::UpdateCheck, SettingValue::Bool(on)) => {
                self.settings.set_bool("update", "check", on);
            }
            (Setting::UpdateChannel, SettingValue::Index(index)) => {
                if let Some(channel) = UpdateChannel::ALL.get(index) {
                    self.settings.set_value("update", "channel", channel.key());
                }
            }
            (Setting::CloudSlots, SettingValue::Index(index)) => {
                self.settings.set_value("predict", "slots", index as i64);
            }
            (Setting::Traditional, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "traditional", on);
            }
            (Setting::EnglishCandidates, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "english_candidates", on);
            }
            (Setting::ChineseFirst, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "chinese_first", on);
            }
            (Setting::EnglishInChinese, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "english_in_chinese", on);
            }
            (Setting::Emoji, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "emoji", on);
            }
            (Setting::ShiftLetter, SettingValue::Bool(on)) => {
                let mode = if on {
                    ShiftLetter::Compose
                } else {
                    ShiftLetter::Passthrough
                };
                self.settings
                    .set_value("general", "shift_letter", mode.key());
            }
            // 勾上寫預設白名單，去掉寫 ["*"]（處處都給）；手改過的名單勾一下就回預設
            (Setting::EnglishCandidatesOnlyInApps, SettingValue::Bool(on)) => {
                let apps: toml_edit::Array = if on {
                    DEFAULT_ENGLISH_CANDIDATES_ON.iter().copied().collect()
                } else {
                    ["*"].into_iter().collect()
                };
                self.settings
                    .set_value("apps", "english_candidates_on", apps);
            }
            // 彈出選單按 Scheme::ALL 的順序。寫的是 [general] scheme（舊鍵 shuangpin 已併入它）：
            // 寫舊鍵的話，配置裡 scheme 的預設值非空、解析時優先，使用者選的方案會被靜默忽略。
            (Setting::Scheme, SettingValue::Index(index)) => {
                let key = Scheme::ALL
                    .get(index)
                    .map_or(Scheme::Pinyin.key(), |scheme| scheme.key());
                self.settings.set_value("general", "scheme", key);
            }
            (Setting::ShuangpinRawPreedit, SettingValue::Bool(on)) => {
                self.settings
                    .set_bool("general", "shuangpin_raw_preedit", on);
            }
            // 五筆：勾上就是 86 版，取消就是關。與上面的拼音方案同時開著就是混輸。
            (Setting::Wubi, SettingValue::Bool(on)) => {
                self.settings
                    .set_value("general", "wubi", if on { "wubi86" } else { "" });
            }
            // 文本框失焦也會發 action：值沒變就不寫，免得每次切視窗都重寫一遍配置
            (Setting::BaseUrl, SettingValue::Text(text)) => {
                let text = text.trim();
                if !text.is_empty() && text != config.predict.base_url {
                    self.settings.set_value("predict", "base_url", text);
                }
            }
            (Setting::Model, SettingValue::Text(text)) => {
                let text = text.trim();
                if !text.is_empty() && text != config.predict.model {
                    self.settings.set_value("predict", "model", text);
                }
            }
            (Setting::ApiKey, SettingValue::Text(text)) => {
                let text = text.trim();
                // 密碼框看不見內容，貼上多了（帶上了終端提示符、命令）使用者發現不了；這種值寫進 .env 還會讓整個檔案解析失敗
                if text.chars().any(|c| !c.is_ascii_graphic()) {
                    self.preferences.set_status(
                        "密钥没有保存：里面有空格或非英文字符，多半是粘贴时多带了别的内容",
                    );
                    return;
                }
                if text.is_empty() {
                    return;
                }
                if self.settings.set_env_var(&config.predict.api_key_env, text) {
                    // 金鑰換了必須重建 Predictor
                    self.apply_config(true);
                    self.preferences.set_status("密钥已保存");
                } else {
                    self.preferences
                        .set_status("密钥没有保存：写不进配置目录的 .env，详情见日志");
                }
                return;
            }
            (Setting::TestCloud, _) => {
                self.start_cloud_test();
                return;
            }
            (Setting::OpenConfigFile, _) => {
                if let Some(path) = self.settings.path() {
                    open_with_system(&["-t", &path.to_string_lossy()]);
                }
                return;
            }
            (Setting::InputLog, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "input_log", on);
            }
            (Setting::Learning, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "learning", on);
            }
            (Setting::SystemTextReplacements, SettingValue::Bool(on)) => {
                self.settings
                    .set_bool("general", "system_text_replacements", on);
            }
            (Setting::ClearInputLog, _) => {
                self.clear_input_log();
                return;
            }
            (Setting::VerboseLog, SettingValue::Bool(on)) => {
                let level = if on { LogLevel::Debug } else { LogLevel::Info };
                self.settings.set_value("general", "log_level", level.key());
            }
            (Setting::CheckUpdateNow, _) => {
                if let Some(updates) = &self.updates {
                    updates.check_now(&self.settings.config().update);
                }
                self.sync_update();
                return;
            }
            (Setting::OpenDownload, _) => {
                open_with_system(&[qingjian_update::DOWNLOAD_URL]);
                return;
            }
            (Setting::OpenWebsite, _) => {
                open_with_system(&[crate::preferences::WEBSITE_URL]);
                return;
            }
            (Setting::OpenRepository, _) => {
                open_with_system(&[crate::preferences::REPOSITORY_URL]);
                return;
            }
            (Setting::OpenLogDirectory, _) => {
                if let Some(dir) = logging::log_dir() {
                    open_with_system(&[&dir.to_string_lossy()]);
                }
                return;
            }
            (Setting::CopyDiagnostics, _) => {
                copy_to_pasteboard(&self.diagnostics());
                self.preferences
                    .set_status("诊断信息已复制到剪贴板，粘贴给作者即可");
                return;
            }
            (Setting::ExportLogs, _) => {
                match logging::export_logs() {
                    Ok(zip) => {
                        open_with_system(&["-R", &zip.to_string_lossy()]);
                        self.preferences
                            .set_status("日志已打包到桌面，发给作者即可");
                    }
                    Err(error) => self
                        .preferences
                        .set_status(&format!("打包日志失败：{error}")),
                }
                return;
            }
            (setting, value) => tracing::warn!(?setting, ?value, "设置项与控件值不匹配"),
        }
        self.apply_config(false);
    }

    /// 字號文本框（`style` 是 `candidate` / `annotation`）：框裡沒設過時顯示主題的字號，清空或填 0 回到主題的，不限範圍；
    /// 填的不是數就不寫（隨後的同步把框裡改回原值）。
    fn set_font_size(&mut self, style: &str, text: &str, current: FontSize) {
        let text = text.trim();
        let input = if text.is_empty() {
            None
        } else if let Ok(size) = text.parse::<f64>() {
            Some(size)
        } else {
            return;
        };
        let theme_id = self.settings.config().general.theme_id().to_owned();
        let theme_size = self.themes.resolve(&theme_id, false).theme_size(style);
        if let Some(size) = current.edited(input, theme_size) {
            self.settings
                .set_value("general", &format!("{style}_font_size"), size);
        }
    }
}
