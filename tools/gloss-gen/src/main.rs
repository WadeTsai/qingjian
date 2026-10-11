//! 用 LLM 批次生成中文詞的英文 / 日文 / 越南文釋義表（詞性 + 譯詞 + 日文假名讀音），以及給多音字詞標拼音。
//!
//! `generate` 從一元詞頻表挑詞、分批問模型，結果逐行追加到 JSONL（中斷了再跑會跳過已完成的詞）；
//! `export` 把 JSONL 轉成輸入法載入的 `glossary-en.tsv` / `glossary-ja.tsv` / `glossary-vi.tsv`；
//! `pinyin` 給詞表裡的詞標拼音（建詞庫時含多音字的詞靠它定讀音），結果 `pinyin-llm.jsonl`；
//! `english` / `export-english` 給英文詞寫中文釋義，匯出 `glossary-zh.tsv`（英文候選右側顯示）。
//! 金鑰來自 `--api-key` 或環境變數 `QINGJIAN_API_KEY`（也讀當前目錄的 `.env`）。

mod args;
mod batch;
mod client;
mod english;
mod entry;
mod error;
mod generate;
mod pinyin;
mod prompt;
mod store;
mod words;

use clap::Parser;

use crate::args::{Args, Command};
use crate::error::GlossError;

fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    if let Err(error) = run() {
        tracing::error!(%error, "失败");
        std::process::exit(1);
    }
}

fn run() -> Result<(), GlossError> {
    let args = Args::parse();
    match args.command {
        Command::Generate(generate) => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            runtime.block_on(generate::run(generate))
        }
        Command::Export(export) => store::export(&export.input, &export.out_dir),
        Command::English(english) => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            runtime.block_on(english::run(english))
        }
        Command::ExportEnglish(export) => store::export_english(&export.input, &export.out_dir),
        Command::Pinyin(pinyin) => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            runtime.block_on(pinyin::run(pinyin))
        }
    }
}
