//! `mmh-reference` 子命令的引數與預設值。clap 定義就在這裡（`Command::MmhReference` 裝整個結構體）。

use std::path::PathBuf;

use clap::Args;

/// `mmh-reference` 子命令的引數。
#[derive(Debug, Args)]
pub struct MmhReferenceOptions {
    /// hanzi-writer-data 解包目錄（每字一個 `<字>.json`，含 medians）
    #[arg(long, default_value = "data/mmh/package")]
    pub mmh: PathBuf,

    /// 字表：只出表裡的字，按表序排列
    #[arg(long, default_value = "assets/lexicon/01_characters/level1_3500.tsv")]
    pub filter: PathBuf,

    /// 對照產物目錄（gitignore，不進倉庫）：寫 prc-counts-l1.tsv 與 prc-first-strokes-l1.tsv
    #[arg(long, default_value = "data/mmh")]
    pub out_dir: PathBuf,
}
