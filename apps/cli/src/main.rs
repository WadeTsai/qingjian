//! 青簡 CLI：Phase 1 的測試工具。
//!
//! 輸入拼音，列印候選（詞性 + 譯文）和各階段耗時；輸入序號上屏並記入使用者詞頻。
//! 不依賴任何平臺 API，是 Core 的第一個「殼」。

mod args;
mod cold;
mod display;
mod error;
mod eval;
mod logging;
mod repl;
mod replay;
mod rescoring;
mod tuning;

use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use qingjian_core::{EmojiTable, Engine, FuzzyRules, Language};
use qingjian_dictionary::{AuxCodeLookup, AuxCodeTable, CodeTable, Dictionary, WordList};
use qingjian_learning::FrequencyLearner;
use qingjian_lm::BigramModel;
use qingjian_platform::{Config, Scheme};
use qingjian_predict::CloudPredictor;
use qingjian_translate::Glossary;

use crate::args::Args;
use crate::error::CliError;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), CliError> {
    dotenvy::dotenv().ok();
    let args = Args::parse();
    let _log_guard = logging::init()?;

    let started = Instant::now();
    let mut engine = build_engine(&args)?;
    tracing::info!(total_ms = started.elapsed().as_millis(), "Engine 就绪");
    engine.set_english_mode(args.english_mode);
    engine.set_chinese_first(args.chinese_first);
    tuning::apply(&mut engine, &args.tune)?;
    if let Some(input) = &args.eval_cold {
        cold::run(
            &mut engine,
            input,
            args.cold_output.as_deref().expect("required by clap"),
            args.cold_model.as_deref(),
        )?;
        return Ok(());
    }
    // 查碼：只看碼錶，不查詞、不進互動
    if !args.aux_query.is_empty() {
        for word in &args.aux_query {
            let codes: Vec<&str> = engine
                .aux_codes()
                .iter()
                .flat_map(|table| table.codes_of(word))
                .collect();
            if codes.is_empty() {
                println!("{word}\t（没有码）");
            } else {
                println!("{word}\t{}", codes.join(" "));
            }
        }
        return Ok(());
    }
    if let Some(path) = &args.replay {
        // 日誌裡形碼那些行也要能重放：回放按每條的方案切引擎，所以它自己得留一份碼錶
        // （`build_engine` 那份的所有權已經交給引擎了）
        let code_table = args.wubi.as_ref().map(CodeTable::from_path).transpose()?;
        let report = replay::run(&mut engine, path, args.misses, code_table)?;
        print!("{report}");
        return Ok(());
    }
    if let Some(model) = &args.eval_generate {
        let report = eval::generate::run(
            &engine,
            &args.eval_text,
            model,
            args.eval_details.as_deref(),
        )?;
        print!("{report}");
        return Ok(());
    }
    if !args.eval_text.is_empty() {
        let report = eval::run(
            &mut engine,
            &args.eval_text,
            args.eval_save.as_deref(),
            args.misses,
            args.eval_details.as_deref(),
        )?;
        print!("{report}");
        return Ok(());
    }
    if args.inputs.is_empty() {
        repl::run(&mut engine, args.limit)?;
    } else {
        for input in &args.inputs {
            println!("> {input}");
            if args.typing {
                display::show_typing(&mut engine, input);
            } else {
                display::show(&mut engine, input, args.limit);
            }
        }
    }
    engine.learner_mut().flush();
    Ok(())
}

/// 組裝 Engine：這是 Core 之外唯一知道具體 Translator / Learner 型別的地方。
fn build_engine(args: &Args) -> Result<Engine, CliError> {
    let language: Language = args
        .language
        .parse()
        .map_err(|_| CliError::Language(args.language.clone()))?;
    if language == Language::Chinese {
        return Err(CliError::Language(args.language.clone()));
    }
    let dict_path = args
        .dict
        .clone()
        .unwrap_or_else(|| args::default_data_file("dict.tsv"));
    let glossary_path = args
        .glossary
        .clone()
        .unwrap_or_else(|| args::default_data_file(&format!("glossary-{}.tsv", language.code())));

    let started = Instant::now();
    let dictionary = Dictionary::from_path(&dict_path)?;
    let dict_load = started.elapsed();
    let started = Instant::now();
    let glossary = Glossary::from_path(language, &glossary_path)?;
    let glossary_load = started.elapsed();
    let english_path = args.english.clone().or_else(|| {
        let path = args::default_data_file("english.tsv");
        path.is_file().then_some(path)
    });
    let started = Instant::now();
    let english = english_path.as_ref().map(WordList::from_path).transpose()?;
    let english_load = started.elapsed();
    let learner = match &args.user_dict {
        Some(path) => FrequencyLearner::from_path(path)?,
        None => FrequencyLearner::default(),
    };
    tracing::info!(
        dict = %dict_path.display(),
        entries = dictionary.len(),
        glossary = %glossary_path.display(),
        glosses = glossary.len(),
        english = english.as_ref().map_or(0, WordList::len),
        learned = learner.len(),
        dict_ms = dict_load.as_millis(),
        glossary_ms = glossary_load.as_millis(),
        english_ms = english_load.as_millis(),
        "加载完成"
    );
    let mut engine = Engine::new(dictionary)
        .with_translator(Box::new(glossary))
        .with_learner(Box::new(learner));
    if !args.extra_dict.is_empty() {
        let mut extras = Vec::new();
        for path in &args.extra_dict {
            let dictionary = Dictionary::from_path(path)?;
            tracing::info!(path = %path.display(), entries = dictionary.len(), "附加词库已加载");
            extras.push(dictionary);
        }
        engine.set_extra_dictionaries(extras);
    }
    // 英文候選的中文釋義可選
    let zh_glossary = args::default_data_file("glossary-zh.tsv");
    if zh_glossary.is_file() {
        let glossary = Glossary::from_path(Language::Chinese, &zh_glossary)?;
        tracing::info!(glosses = glossary.len(), "英→中释义表已加载");
        engine = engine.with_english_translator(Box::new(glossary));
    }
    if let Some(words) = english {
        engine = engine.with_english(words);
    }
    // emoji 表隨倉庫提供（Unicode License）：中文表 + 英文表合成一張，一張都沒有就不出 emoji 候選
    let mut emoji: Option<EmojiTable> = None;
    let started = Instant::now();
    for name in ["emoji-zh.tsv", "emoji-en.tsv"] {
        let path = std::path::PathBuf::from("assets/emoji").join(name);
        if !path.is_file() {
            continue;
        }
        let table = EmojiTable::from_path(&path)?;
        match &mut emoji {
            Some(all) => all.merge(table),
            None => emoji = Some(table),
        }
    }
    if let Some(table) = emoji {
        tracing::info!(
            words = table.len(),
            load_ms = started.elapsed().as_millis(),
            "emoji 表已加载"
        );
        engine = engine.with_emoji(table);
    }
    // 語言模型可選：沒有就退化成一元詞頻整句；打包過的 lm.qj 優先
    let packed = std::path::PathBuf::from("data/generated/lm.qj");
    let unigram = std::path::PathBuf::from("data/generated/lm-unigram.tsv");
    let bigram = std::path::PathBuf::from("data/generated/lm-bigram.tsv");
    if packed.is_file() || (unigram.is_file() && bigram.is_file()) {
        let started = Instant::now();
        let model = if packed.is_file() {
            BigramModel::from_path(&packed)?
        } else {
            BigramModel::from_paths(&unigram, &bigram)?
        };
        tracing::info!(
            words = model.word_count(),
            bigrams = model.bigram_count(),
            load_ms = started.elapsed().as_millis(),
            "语言模型已加载"
        );
        engine = engine.with_language_model(Box::new(model));
    }
    if let Some(dir) = &args.neural {
        let started = Instant::now();
        let scorer = qingjian_neural::CharScorer::load(dir)?;
        // 與產品端一致：字表裡有 <sep> 的是 P2C 模型，按按鍵打分；沒有的是字級模型，按前文打分
        let p2c = scorer.vocab().sep().is_some();
        let scorer: Box<dyn qingjian_core::sentence::SentenceScorer> = if p2c {
            Box::new(qingjian_neural::P2cScorer(scorer))
        } else {
            Box::new(scorer)
        };
        tracing::info!(
            load_ms = started.elapsed().as_millis(),
            weight = args.neural_weight.unwrap_or(qingjian_core::NEURAL_WEIGHT),
            kind = if p2c { "P2C" } else { "字级" },
            "神经重打分已启用"
        );
        engine = if args.neural_async {
            engine.with_async_sentence_scorer(
                scorer,
                args.neural_weight,
                args.neural_margin,
                args.neural_context,
            )
        } else {
            engine.with_sentence_scorer(
                scorer,
                args.neural_weight,
                args.neural_margin,
                args.neural_context,
            )
        };
    }
    if let Some(path) = &args.eval_p2c {
        let scorer = qingjian_neural::CharScorer::load(path)?;
        if scorer.vocab().sep().is_none() {
            return Err(qingjian_neural::NeuralError::Corrupt(
                "Hanzhang Tongbian model requires a <sep> token in its vocabulary",
            )
            .into());
        }
        engine = engine.with_sentence_scorer(
            Box::new(eval::p2c::P2cScorer(scorer)),
            args.neural_weight,
            args.neural_margin,
            None,
        );
    }
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(args::default_config_file);
    let mut config = if args.eval_cold.is_some() {
        let mut isolated = Config::default();
        isolated.predict.enabled = false;
        isolated
    } else {
        Config::load(&config_path)?
    };
    if args.predict {
        config.predict.enabled = true;
    }
    if !args.fuzzy.is_empty() {
        let mut rules = FuzzyRules::default();
        for name in &args.fuzzy {
            if name == "all" {
                rules = FuzzyRules::ALL;
            } else if !rules.enable(name) {
                tracing::warn!(name, "不认识的模糊音规则，忽略");
            }
        }
        config.fuzzy = rules;
    }
    if config.fuzzy.any() {
        tracing::info!(rules = ?config.fuzzy, "模糊音已启用");
    }
    engine.set_traditional_mode(config.general.traditional);
    engine.set_fuzzy(config.fuzzy);
    engine.set_mode_keys(config.shortcut.mode);
    // `--shuangpin` 現在寫的是 [general] scheme（同一個維度的舊鍵已經並進去），off 就是全拼
    if let Some(scheme) = &args.shuangpin {
        config.general.scheme = if scheme == "off" {
            Scheme::Pinyin.key().to_owned()
        } else {
            scheme.clone()
        };
    }
    // 兩條軸：拼音側看 `[general] scheme`，形碼側看 `[general] wubi`；`--wubi` 給了碼錶就算開著形碼。
    // 兩邊都開就是混輸，見 docs/user/input/fuzzy-and-shuangpin.md
    let scheme = config.general.scheme();
    let wubi = config.general.wubi() || args.wubi.is_some();
    tracing::info!(pinyin = scheme.key(), wubi, "输入方案已启用");
    engine.set_shuangpin(scheme.shuangpin());
    engine.set_zhuyin_mode(scheme == Scheme::Zhuyin);
    // 拼音側關掉且形碼開著才是「只用形碼」；兩邊都關著時留拼音兜底
    engine.set_phonetic(scheme.is_on() || !wubi);
    // 形碼的碼錶由 `--wubi` 顯式給（方案本身只說「用哪套」，碼錶檔案在哪由殼決定）
    if let Some(path) = &args.wubi {
        engine.set_code_table(Some(CodeTable::from_path(path)?));
        tracing::info!(table = %path.display(), "形码码表已载入");
    }
    engine.set_aux_code_key(config.general.aux_code_key(), config.general.page_keys());
    engine.set_aux_keep_empty(config.general.aux_code_keep_empty);
    if !args.aux_table.is_empty() {
        let mut tables: Vec<Arc<dyn AuxCodeLookup>> = Vec::new();
        for path in &args.aux_table {
            let table = AuxCodeTable::from_path(path)?;
            tracing::info!(
                path = %path.display(),
                entries = table.len(),
                words = table.word_count(),
                "辅码码表已加载"
            );
            tables.push(Arc::new(table));
        }
        engine.set_aux_codes(tables);
        // CLI 沒有配置開關：給了碼錶即開輔碼（預設關），replay 統計不啞
        engine.set_aux_enabled(true);
    }
    if config.predict.enabled {
        let predictor = CloudPredictor::new(&config.predict)?;
        engine = engine.with_predictor(Box::new(predictor));
    }
    Ok(engine)
}
