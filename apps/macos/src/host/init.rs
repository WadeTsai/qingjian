//! 啟動：載入詞庫 / 語言模型 / 釋義表 / 學習資料，建 Engine 與候選視窗，裝進執行緒區域性的 HOST。

use super::*;

/// 載入資料並建立單例。必須在主執行緒、在 IMKServer 建立之前呼叫。版本顯示在選單末行與「關於」頁。
pub fn init(mtm: MainThreadMarker, info: &BundleInfo) -> Result<(), HostError> {
    let version = info.version.as_str();
    let settings = Settings::load();
    let started = std::time::Instant::now();
    // 打包過的 .qj 直接對映；沒有就解析 TSV（樣例詞庫）
    let dictionary = Dictionary::from_path(
        paths::resource("dict.qj").or_else(|_| paths::resource("dict.tsv"))?,
    )?;
    let dictionary_ms = started.elapsed().as_millis();
    let languages: Vec<Language> = GLOSSARY_LANGUAGES
        .into_iter()
        .filter(|language| glossary_path(*language).is_ok())
        .collect();
    // 寫 off 就關；配置裡的學習語言沒有對應釋義表時退回第一種有的，一種都沒有也當關
    let general = &settings.config().general;
    let learning_language = if general.learning_language_off() {
        None
    } else {
        general
            .learning_language
            .parse::<Language>()
            .ok()
            .filter(|language| languages.contains(language))
            .or_else(|| languages.first().copied())
    };
    let glossary = learning_language.map(load_glossary).transpose()?;
    let learner = match paths::user_data_dir() {
        Some(dir) => load_learner(&dir),
        None => FrequencyLearner::default(),
    };
    // 英文詞表可選：沒有就不出英文候選
    let english = paths::resource("english.tsv")
        .ok()
        .map(WordList::from_path)
        .transpose()?;
    tracing::info!(
        entries = dictionary.len(),
        glosses = glossary.as_ref().map_or(0, LayeredTranslator::len),
        english = english.as_ref().map_or(0, WordList::len),
        learned = learner.len(),
        dictionary_ms,
        "数据加载完成"
    );
    let mut engine = Engine::new(dictionary).with_learner(Box::new(learner));
    if let Some(glossary) = glossary {
        engine = engine.with_translator(Box::new(glossary));
    }
    // 輸入統計（打了多少字）：與學習資料同目錄；沒有資料目錄就只在記憶體裡數
    if let Some(dir) = paths::user_data_dir() {
        // 詞彙等級表（levels-en.tsv / levels-ja.tsv）隨包可選：有就按級統計
        let mut vocabulary = VocabularyBook::open(dir.join(VOCABULARY_FILE));
        for language in GLOSSARY_LANGUAGES {
            let Ok(path) = paths::resource(&format!("levels-{}.tsv", language.code())) else {
                continue;
            };
            match LevelTable::from_path(&path) {
                Ok(table) => vocabulary = vocabulary.with_levels(language, table),
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "词汇等级表读不了，不分级")
                }
            }
        }
        engine = engine
            .with_usage_meter(Box::new(UsageStats::open(dir.join(USAGE_FILE))))
            .with_vocabulary_tracker(Box::new(vocabulary));
    }
    // 附加詞庫：隨包的領域詞庫 + 使用者目錄 dicts/ 下的檔案
    engine.set_extra_dictionaries(extra_dictionaries::load(
        paths::bundled_dicts_dir().as_deref(),
        paths::dicts_dir().as_deref(),
        &settings.config().dictionaries,
    ));
    // 英文候選的中文釋義（英→中）可選：沒有這張表英文候選右側就留空
    if let Ok(path) =
        paths::resource("glossary-zh.qj").or_else(|_| paths::resource("glossary-zh.tsv"))
    {
        match Glossary::from_path(Language::Chinese, &path) {
            Ok(glossary) => {
                tracing::info!(glosses = glossary.len(), "英→中释义表已加载");
                engine = engine.with_english_translator(Box::new(glossary));
            }
            Err(error) => tracing::warn!(%error, "英→中释义表加载失败"),
        }
    }
    if let Some(words) = english {
        engine = engine.with_english(words);
    }
    if let Some(table) = load_emoji_tables(&["emoji-zh.tsv", "emoji-en.tsv"]) {
        tracing::info!(words = table.len(), "emoji 表已加载");
        engine = engine.with_emoji(table);
    }
    // 語言模型可選：沒有就退化成一元詞頻整句；打包過的 lm.qj 優先
    let model = if let Ok(packed) = paths::resource("lm.qj") {
        Some(BigramModel::from_path(&packed)?)
    } else if let (Ok(unigram), Ok(bigram)) = (
        paths::resource("lm-unigram.tsv"),
        paths::resource("lm-bigram.tsv"),
    ) {
        Some(BigramModel::from_paths(&unigram, &bigram)?)
    } else {
        None
    };
    if let Some(model) = model {
        tracing::info!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            total_ms = started.elapsed().as_millis(),
            "语言模型已加载"
        );
        engine = engine.with_language_model(Box::new(model));
    }
    let window = CandidateWindow::new(mtm);
    window.set_click_handler(Box::new(crate::imk::click));
    let indicator = ModeIndicator::new(mtm);
    let menu = InputMenu::new(mtm, version);
    indicator.set_menu(&menu.ns_menu());
    let preferences = PreferencesWindow::new(mtm, &languages, version, &info.build);
    let monitor = PredictMonitor::new(mtm);
    let watch = ConfigWatch::new(mtm);
    HOST.with(|host| {
        *host.borrow_mut() = Some(Host {
            engine,
            window,
            indicator,
            menu,
            preferences,
            settings,
            watch,
            themes: qingjian_render::ThemeLibrary::load(paths::themes_dir().as_deref()),
            last_flush: std::time::Instant::now(),
            applied_predict: PredictConfig::default(),
            applied_dictionaries: DictionariesConfig::default(),
            dictionary_list: Vec::new(),
            learning_language,
            languages,
            version: version.to_owned(),
            build: info.build.clone(),
            page_size: 9,
            cloud_slots: 2,
            page_keys: qingjian_platform::DEFAULT_PAGE_KEYS,
            punct_commits: false,
            translation_keys: ShortcutConfig::default().translation_keys(),
            delete_keys: ShortcutConfig::default().delete_keys(),
            status: None,
            input_log_enabled: None,
            translate_keys: KeyCombo::TRANSLATE_DEFAULT,
            translation: None,
            notice: None,
            preedit_mode: PreeditMode::default(),
            layout: LayoutMode::default(),
            horizontal_grid: false,
            english_candidates: true,
            text_replacements: Vec::new(),
            apps: AppsConfig::default(),
            monitor,
            cloud_test: None,
            cloud_test_monitor: CloudTestMonitor::new(mtm),
            rescore: RescoreMonitor::new(mtm),
            model_loader: None,
            applied_model: None,
            updates: paths::user_data_dir()
                .map(|dir| qingjian_update::Checker::new(dir.join(UPDATE_STATE_FILE), version)),
            update_status: UpdateStatus::default(),
            session: Session::default(),
            sentence: None,
            anchor: NSRect::ZERO,
        })
    });
    // 配置裡的開關走和選單 / 設定視窗 / 熱載入同一條通路
    with(|host| host.apply_config(false));
    Ok(())
}

/// 讀使用者的學習資料。格式壞掉的行學習 crate 自己跳過；真讀不了（許可權、壞盤）就退回只在記憶體裡學，
/// 輸入法照常啟動，也不會拿空表覆蓋使用者的檔案。學習資料出問題不能讓輸入法起不來。
pub(super) fn load_learner(dir: &std::path::Path) -> FrequencyLearner {
    let path = dir.join("user.tsv");
    match FrequencyLearner::from_path(&path) {
        Ok(learner) => learner,
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "学习数据读取失败，本次只在内存里学习");
            FrequencyLearner::default()
        }
    }
}

/// 讀一本詞庫的顯示資訊；檔案壞了標 `broken`。
pub(super) fn dictionary_info(
    stem: String,
    path: PathBuf,
    builtin: bool,
    enabled: bool,
) -> DictionaryInfo {
    match Dictionary::from_path(&path) {
        Ok(dictionary) => DictionaryInfo {
            name: dictionary
                .metadata()
                .map_or(stem.clone(), |m| m.name.clone()),
            entries: dictionary.len(),
            license: dictionary
                .metadata()
                .map_or(String::new(), |m| m.license.clone()),
            enabled,
            broken: false,
            stem,
            builtin,
            path,
        },
        Err(_) => DictionaryInfo {
            name: stem.clone(),
            entries: 0,
            license: String::new(),
            enabled,
            broken: true,
            stem,
            builtin,
            path,
        },
    }
}

/// 把 `stem` 加進（`present` 為真）或移出字串列表；沒變化返回 `None`。
pub(super) fn toggle_membership(
    mut list: Vec<String>,
    stem: &str,
    present: bool,
) -> Option<Vec<String>> {
    let has = list.iter().any(|d| d == stem);
    if present && !has {
        list.push(stem.to_owned());
    } else if !present && has {
        list.retain(|d| d != stem);
    } else {
        return None;
    }
    Some(list)
}

pub(super) fn string_array(items: &[String]) -> toml_edit::Array {
    items.iter().map(String::as_str).collect()
}

/// 把包裡有的 emoji 表（中文、英文）合成一張；一張都沒有返回 `None`，壞了的只記日誌跳過。
pub(super) fn load_emoji_tables(names: &[&str]) -> Option<EmojiTable> {
    let mut merged: Option<EmojiTable> = None;
    for name in names {
        let Ok(path) = paths::resource(name) else {
            continue;
        };
        match EmojiTable::from_path(&path) {
            Ok(table) => match &mut merged {
                Some(all) => all.merge(table),
                None => merged = Some(table),
            },
            Err(error) => tracing::warn!(%error, name, "emoji 表加载失败，跳过"),
        }
    }
    merged
}

pub(super) fn glossary_path(language: Language) -> Result<PathBuf, HostError> {
    paths::resource(&format!("glossary-{}.qj", language.code()))
        .or_else(|_| paths::resource(&format!("glossary-{}.tsv", language.code())))
}

/// 隨包釋義表疊上使用者目錄的個人釋義表（`user-glossary-<語言>.tsv`，釋義兜底寫入、可手改）。
pub(super) fn load_glossary(language: Language) -> Result<LayeredTranslator, HostError> {
    let bundled = Glossary::from_path(language, glossary_path(language)?)?;
    let personal = match paths::user_data_dir() {
        Some(dir) => PersonalGlossary::open(
            language,
            dir.join(format!("user-glossary-{}.tsv", language.code())),
        ),
        None => PersonalGlossary::in_memory(language),
    };
    if !personal.is_empty() {
        tracing::info!(
            language = language.code(),
            entries = personal.len(),
            "个人释义表已加载"
        );
    }
    Ok(LayeredTranslator::new(bundled, personal))
}
