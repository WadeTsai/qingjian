//! 裝配 Engine：Server 裡唯一知道具體 Translator / Learner 型別的地方，裝的東西與 macOS 的 `host::init` 一致。

mod language_model;
mod spec;

use std::path::{Path, PathBuf};
use std::time::Instant;

use qingjian_core::{EmojiTable, Engine, Language};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, InputLog, UsageStats, VocabularyBook};
use qingjian_platform::{Config, code_tables, extra_dictionaries};
use qingjian_translate::{Glossary, LayeredTranslator, LevelTable, PersonalGlossary};

use crate::error::ServerError;

pub use self::language_model::LanguageModelFiles;
pub use self::spec::AssemblySpec;

pub fn assemble(spec: &AssemblySpec) -> Result<Engine, ServerError> {
    let started = Instant::now();
    let dictionary = Dictionary::from_path(&spec.dict)?;
    let learner = match &spec.user_dir {
        Some(dir) => load_learner(dir),
        None => FrequencyLearner::default(),
    };
    tracing::info!(
        entries = dictionary.len(),
        learned = learner.len(),
        dictionary_ms = started.elapsed().as_millis(),
        "詞庫與學習資料已載入"
    );
    let mut engine = Engine::new(dictionary).with_learner(Box::new(learner));
    if let Some((language, path)) = &spec.glossary {
        engine = engine.with_translator(Box::new(load_glossary(
            *language,
            path,
            spec.user_dir.as_deref(),
        )?));
    }
    if let Some(dir) = &spec.user_dir {
        engine = engine
            .with_usage_meter(Box::new(UsageStats::open(dir.join("usage.tsv"))))
            .with_vocabulary_tracker(Box::new(load_vocabulary(dir, spec.levels_dir.as_deref())));
        if spec.input_log {
            let path = dir.join("input-log.jsonl");
            tracing::info!(path = %path.display(), "輸入日誌開著");
            engine = engine.with_input_logger(Box::new(InputLog::open(path)));
        }
    }
    engine.set_extra_dictionaries(extra_dictionaries::load(
        spec.bundled_dicts_dir.as_deref(),
        user_dicts_dir(spec.user_dir.as_deref()).as_deref(),
        &spec.dictionaries,
    ));
    engine.set_aux_codes(code_tables::load(
        spec.bundled_codes_dir.as_deref(),
        user_codes_dir(spec.user_dir.as_deref()).as_deref(),
        &spec.aux_code,
    ));
    if let Some(path) = &spec.english_glossary {
        match Glossary::from_path(Language::Chinese, path) {
            Ok(glossary) => {
                tracing::info!(glosses = glossary.len(), "英→中釋義表已載入");
                engine = engine.with_english_translator(Box::new(glossary));
            }
            Err(error) => tracing::warn!(%error, "英→中釋義表載入失敗"),
        }
    }
    if let Some(path) = &spec.english {
        let words = WordList::from_path(path)?;
        tracing::info!(words = words.len(), "英文詞表已載入");
        engine = engine.with_english(words);
    }
    if let Some(table) = load_emoji(&spec.emoji) {
        tracing::info!(words = table.len(), "emoji 表已載入");
        engine = engine.with_emoji(table);
    }
    if let Some(files) = &spec.language_model {
        let started = Instant::now();
        let model = files.load()?;
        tracing::info!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            load_ms = started.elapsed().as_millis(),
            "語言模型已載入"
        );
        engine = engine.with_language_model(Box::new(model));
    }
    Ok(engine)
}

/// 使用者匯入詞庫目錄 `dicts/`，不存在則建立；建不了當沒有。
pub fn user_dicts_dir(user_dir: Option<&Path>) -> Option<std::path::PathBuf> {
    let dir = user_dir?.join("dicts");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 使用者匯入碼錶目錄 `codes/`，不存在則建立；建不了當沒有。
pub fn user_codes_dir(user_dir: Option<&Path>) -> Option<std::path::PathBuf> {
    let dir = user_dir?.join("codes");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 讀不了就退回只在記憶體裡學，不拿空表覆蓋使用者檔案。
fn load_learner(dir: &Path) -> FrequencyLearner {
    let path = dir.join("user.tsv");
    match FrequencyLearner::from_path(&path) {
        Ok(learner) => learner,
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "學習資料讀取失敗，本次只在記憶體裡學習");
            FrequencyLearner::default()
        }
    }
}

/// 配置裡的學習語言；`off` 為 `None`（不顯示譯文），寫得不認識按英文。
pub fn learning_language(config: &Config) -> Option<Language> {
    if config.general.learning_language_off() {
        return None;
    }
    let code = &config.general.learning_language;
    Some(code.parse().unwrap_or_else(|_| {
        tracing::warn!(code, "不認識的學習語言，按英文");
        Language::English
    }))
}

/// 某語言的釋義表：`<root>/data/generated/` 打包過的 `.qj` 優先，否則隨 git 的 `assets/glossary/` TSV；都沒有為 `None`。
pub fn glossary_file(root: &Path, language: Language) -> Option<PathBuf> {
    let code = language.code();
    [
        root.join("data/generated")
            .join(format!("glossary-{code}.qj")),
        root.join("assets/glossary")
            .join(format!("glossary-{code}.tsv")),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// 隨包釋義表疊上個人釋義表（`user-glossary-<語言>.tsv`）。啟動裝配與熱載入換語言共用。
pub(crate) fn load_glossary(
    language: Language,
    path: &Path,
    user_dir: Option<&Path>,
) -> Result<LayeredTranslator, ServerError> {
    let bundled = Glossary::from_path(language, path)?;
    let personal = match user_dir {
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
            "個人釋義表已載入"
        );
    }
    Ok(LayeredTranslator::new(bundled, personal))
}

/// 詞彙記錄（`user-vocab.tsv`），有等級表就按級統計。
fn load_vocabulary(user_dir: &Path, levels_dir: Option<&Path>) -> VocabularyBook {
    let mut vocabulary = VocabularyBook::open(user_dir.join("user-vocab.tsv"));
    let Some(levels_dir) = levels_dir else {
        return vocabulary;
    };
    for language in [
        Language::English,
        Language::Japanese,
        Language::Spanish,
        Language::Vietnamese,
    ] {
        let path = levels_dir.join(format!("levels-{}.tsv", language.code()));
        if !path.is_file() {
            continue;
        }
        match LevelTable::from_path(&path) {
            Ok(table) => vocabulary = vocabulary.with_levels(language, table),
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "詞彙等級表讀不了，不分級");
            }
        }
    }
    vocabulary
}

/// 幾張 emoji 表合成一張；壞的跳過。
fn load_emoji(paths: &[std::path::PathBuf]) -> Option<EmojiTable> {
    let mut merged: Option<EmojiTable> = None;
    for path in paths {
        match EmojiTable::from_path(path) {
            Ok(table) => match &mut merged {
                Some(all) => all.merge(table),
                None => merged = Some(table),
            },
            Err(error) => tracing::warn!(%error, path = %path.display(), "emoji 表載入失敗，跳過"),
        }
    }
    merged
}
