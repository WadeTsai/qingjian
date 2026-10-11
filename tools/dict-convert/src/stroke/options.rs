//! stroke 子命令的引數與預設值。clap 定義就在這裡（`Command::Stroke` 裝整個結構體），加引數只動這一處。

use std::path::PathBuf;

use clap::Args;

/// `stroke` 子命令的引數。
#[derive(Debug, Args)]
pub struct StrokeOptions {
    /// CNS 筆順資料（`CNS_strokes_sequence.txt`）：`CNS 字碼<TAB>1-5 序列`
    #[arg(long, default_value = "data/cns/CNS_strokes_sequence.txt")]
    pub cns_seq: PathBuf,

    /// CNS→Unicode 對照表（`CNS2UNICODE_Unicode*.txt`）：給檔案或目錄（目錄取其中的對照表）
    #[arg(long, default_value = "data/cns", num_args = 1..)]
    pub cns_map: Vec<PathBuf>,

    /// 官方筆畫數（`CNS_stroke.txt`）：與序列長度自洽的字才留，不給就不過濾
    #[arg(long)]
    pub cns_count: Option<PathBuf>,

    /// 自洽過濾的容差：序列長度與筆畫數之差超過它的字丟掉
    #[arg(long, default_value_t = 1)]
    pub max_diff: usize,

    /// 字表白名單（預設通用規範字表）：只出表裡的字，按表序排列
    #[arg(long, default_value = "assets/lexicon/01_characters/standard_8105.tsv")]
    pub filter: PathBuf,

    /// 大陸序覆蓋表：部件重寫規則 + 例外字 + 整字補錄
    #[arg(long, default_value = "assets/stroke/prc-rules.tsv")]
    pub prc_rules: PathBuf,

    /// 產物路徑；預設寫到 <輸出目錄>/codes/stroke.tsv
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// 寫完再對照大陸規範：筆畫數抽樣比對、首筆全量比對，白名單之外一處不符就退出碼非 0；
    /// 對照表找不到的那張跳過並提示（mmh-reference 生成）
    #[arg(long)]
    pub verify: bool,

    /// 抽樣用的字表：按表序每 `--stride` 字取一個
    #[arg(long, default_value = "assets/lexicon/01_characters/level1_3500.tsv")]
    pub sample: PathBuf,

    /// 抽樣密度：每幾字取一個
    #[arg(long, default_value_t = 12)]
    pub stride: usize,

    /// 筆畫數對照表（`字<TAB>大陸筆畫數`）：找不到就跳過筆畫數對照
    #[arg(long, default_value = "data/mmh/prc-counts-l1.tsv")]
    pub reference: PathBuf,

    /// 殘留差異白名單（`字<TAB>本表筆畫數<TAB>對照筆畫數<TAB>說明`）
    #[arg(long, default_value = "assets/stroke/residual-whitelist.tsv")]
    pub whitelist: PathBuf,

    /// 首筆對照表（`字<TAB>幾何類別 h/s/p/n/z/?`）：字表全量比對；找不到就跳過首筆對照
    #[arg(long, default_value = "data/mmh/prc-first-strokes-l1.tsv")]
    pub first_reference: PathBuf,

    /// 首筆殘留差異白名單（`字<TAB>本表首筆<TAB>對照類別<TAB>說明`）
    #[arg(long, default_value = "assets/stroke/residual-first-strokes.tsv")]
    pub first_whitelist: PathBuf,
}
