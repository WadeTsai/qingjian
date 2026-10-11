//! 把資料來源轉換成青簡的 TSV 格式，或打包成 `.qj`。
//!
//! - `lexicon`：青簡基礎詞庫，「輸入法字詞庫_分類整理版」資料包（規範字 / 常用詞 / THUOCL 領域詞）+ Unihan 讀音 + LLM 多音字標註 → `dict.tsv`
//! - `wubi`：Rime 形碼碼錶（極點 86 五筆，Apache-2.0）→ `wubi86.tsv`（`詞\t編碼\t詞頻`，詞頻由青簡詞庫按詞面回填）
//! - `cedict`：CC-CEDICT（CC BY-SA 4.0）→ `glossary-en.tsv`（釋義表的備用來源，現在用 gloss-gen 的 LLM 表）
//! - `english`：`詞\t編碼` 英文詞表（資料包的 `05_english`，ESDB / CSpell，MIT）→ `english.tsv`
//! - `emoji`：Unicode CLDR annotations（Unicode License v3，`--language zh|en`）→ `emoji-<語言>.tsv`（可釋出，放 `assets/emoji/`）
//! - `bigram`：純文本語料（如 `tools/corpus/parquet_to_text.py` 轉出的中文維基 CC BY-SA 4.0、LCCC 對話 MIT）→ `lm-unigram.tsv` + `lm-bigram.tsv`
//! - `mine`：語料裡分詞落成連續單字的段 → `oov-candidates.tsv`（詞庫沒收的高頻詞，標音後用 `lexicon --extra-words` 併入）
//! - `phrases`：bigram 表的相鄰兩詞 + 語料的相鄰三詞 → `phrases.tsv`（我的 / 不知道 這類短語層，讀音由成分詞拼出，同樣用 `lexicon --extra-words` 併入）
//! - `stroke`：CNS11643 全字型檔「筆順資料」+ 大陸序覆蓋表（`assets/stroke/prc-rules.tsv`）→ `codes/stroke.tsv`（隨包筆畫碼錶的源資料，`--verify` 抽樣對照大陸筆畫數）
//! - `mmh-reference`：hanzi-writer-data（Make Me a Hanzi；Arphic 許可，不進倉庫）→ `data/mmh/` 的兩張開發期對照表
//!   （筆畫數、首筆幾何類別），`stroke --verify` 找不到哪張就跳過哪張對照
//! - `pack codes`：筆畫表（`stroke` 的產物，`字\t序列`）+ 詞庫 → `codes/stroke.qj`（隨包原生輔碼錶：單字前 4 筆 + 末筆、
//!   片語每字首筆；缺字的詞跳過並計入統計，見 `codes` 模組）
//! - `pack dict|lm|glossary|model`：TSV → `.qj` 容器（`dict.qj` / `lm.qj`），帶名稱 / 許可證 / 署名後設資料，輸入法與 CLI 優先載入它；
//!   `model` 把本地整句模型的三件套目錄打成一個 `.qjm` 檔案
//!
//! 輸出預設寫到倉庫根目錄 `data/generated/`（gitignore）。

mod args;
mod bigram;
mod cedict;
mod codes;
mod emoji;
mod english;
mod error;
mod lexicon;
mod mmh;
mod oov_filter;
mod pack;
mod phrases;
mod stroke;
mod wubi;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::args::{Args, Command};
use crate::error::ConvertError;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), ConvertError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
    let args = Args::parse();
    std::fs::create_dir_all(&args.out_dir)?;
    match args.command {
        Command::Lexicon {
            pack,
            unihan,
            pinyin,
            frequency,
            emit_ambiguous,
            extra_words,
            domain_keep_min,
        } => lexicon::convert(
            &pack,
            &unihan,
            pinyin.as_deref(),
            frequency.as_deref(),
            emit_ambiguous.as_deref(),
            &extra_words,
            domain_keep_min,
            &args.out_dir,
        ),
        Command::Wubi {
            input,
            frequency,
            name,
        } => {
            let converted = wubi::convert(&input, &frequency, &args.out_dir.join(name))?;
            tracing::info!(
                entries = converted.entries,
                from_corpus = converted.with_frequency,
                unknown = converted.unknown,
                "形码码表已写出"
            );
            Ok(())
        }
        Command::Cedict { input } => cedict::convert(&input, &args.out_dir.join("glossary-en.tsv")),
        Command::English { inputs, frequency } => english::convert(
            &inputs,
            frequency.as_deref(),
            &args.out_dir.join("english.tsv"),
        ),
        Command::Emoji { inputs, language } => emoji::convert(
            &inputs,
            &args.out_dir.join(format!("emoji-{language}.tsv")),
            &language,
        ),
        Command::Bigram {
            corpus,
            dict,
            phrases,
            brand,
            min_count,
            max_bigrams,
        } => bigram::convert(
            &corpus,
            &dict,
            &phrases,
            &brand,
            min_count,
            max_bigrams,
            &args.out_dir,
        ),
        Command::Mine {
            corpus,
            dict,
            min_count,
            max_chars,
            frequency,
            min_pmi,
            candidates,
        } => bigram::mine(
            &bigram::MineOptions {
                corpus,
                dict,
                min_count,
                max_chars,
                frequency,
                min_pmi,
                candidates,
            },
            &args.out_dir,
        ),
        Command::Phrases {
            corpus,
            dialogue,
            dict,
            refresh,
            min_count,
            max_chars,
        } => phrases::mine(
            &phrases::PhraseOptions {
                dict,
                refresh,
                corpus,
                dialogue,
                min_count,
                max_chars,
            },
            &args.out_dir,
        ),
        Command::Stroke(options) => stroke::convert(&options, &args.out_dir),
        Command::MmhReference(options) => mmh::generate(&options),
        Command::Pack {
            kind,
            input,
            stroke,
            dict,
            output,
            name,
            license,
            attribution,
            source,
            data_version,
            language,
        } => pack::pack(
            kind,
            &input,
            &pack::CodePaths {
                stroke: stroke.as_deref(),
                dict: dict.as_deref(),
                output: output.as_deref(),
            },
            &language,
            qingjian_format::Metadata {
                name,
                license,
                attribution,
                source,
                version: data_version,
                ..qingjian_format::Metadata::default()
            },
            &args.out_dir,
        ),
    }
}
