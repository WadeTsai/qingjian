use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(about = "用 LLM 批量生成中文词的英文 / 日文 / 越南文释义表，或给多音字词标拼音")]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 從一元詞頻表挑詞，分批問模型，結果追加到 JSONL（可中斷續跑）
    Generate(GenerateArgs),

    /// 把 JSONL 匯出成 glossary-en.tsv / glossary-ja.tsv / glossary-vi.tsv
    Export(ExportArgs),

    /// 給多音字詞標拼音：詞表一行一個詞，結果追加到 JSONL（可中斷續跑），`dict-convert lexicon --pinyin` 讀它
    Pinyin(PinyinArgs),

    /// 英→中釋義：給英文詞表裡的詞寫中文對應詞，結果追加到 JSONL（可中斷續跑）
    English(EnglishArgs),

    /// 把英→中 JSONL 匯出成 glossary-zh.tsv
    ExportEnglish(ExportEnglishArgs),
}

#[derive(Debug, clap::Args)]
pub struct EnglishArgs {
    /// 英文詞表（`詞\t編碼\t詞頻`）
    #[arg(long, default_value = "assets/lexicon/english.tsv")]
    pub words: PathBuf,

    /// 只要詞頻（wordfreq Zipf ×1000）不低於這個值的詞
    #[arg(long, default_value_t = 2500)]
    pub min_frequency: u32,

    /// 這些表（帶表頭的 TSV，第一列是詞）裡的詞不看詞頻一律要，比如技術詞表
    #[arg(long, num_args = 0..)]
    pub include: Vec<PathBuf>,

    /// 只處理前 N 個（試跑用）
    #[arg(long)]
    pub limit: Option<usize>,

    /// 結果 JSONL，已有的詞跳過
    #[arg(long, default_value = "assets/glossary/gloss-en-llm.jsonl")]
    pub out: PathBuf,

    /// 每個請求帶幾個詞
    #[arg(long, default_value_t = 40)]
    pub batch: usize,

    /// 同時幾個請求
    #[arg(long, default_value_t = 8)]
    pub concurrency: usize,

    /// OpenAI 相容介面地址
    #[arg(long, default_value = "https://api.deepseek.com")]
    pub base_url: String,

    /// 模型名
    #[arg(long, default_value = "deepseek-v4-flash")]
    pub model: String,

    /// 金鑰；預設讀環境變數
    #[arg(long, env = "QINGJIAN_API_KEY", hide_env_values = true)]
    pub api_key: String,

    /// 單個請求的超時秒數
    #[arg(long, default_value_t = 180)]
    pub timeout_secs: u64,
}

#[derive(Debug, clap::Args)]
pub struct ExportEnglishArgs {
    /// `english` 寫出的 JSONL
    #[arg(long, default_value = "assets/glossary/gloss-en-llm.jsonl")]
    pub input: PathBuf,

    /// 輸出目錄，寫 glossary-zh.tsv
    #[arg(long, default_value = "assets/glossary")]
    pub out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct PinyinArgs {
    /// 詞表，一行一個詞（`dict-convert lexicon --emit-ambiguous` 寫出的）
    #[arg(long, default_value = "data/generated/lexicon-ambiguous.txt")]
    pub words: PathBuf,

    /// 結果 JSONL，已有的詞跳過
    #[arg(long, default_value = "data/generated/pinyin-llm.jsonl")]
    pub out: PathBuf,

    /// 只處理前 N 個沒做過的詞（試跑用）
    #[arg(long)]
    pub limit: Option<usize>,

    /// 每個請求帶幾個詞
    #[arg(long, default_value_t = 60)]
    pub batch: usize,

    /// 同時幾個請求
    #[arg(long, default_value_t = 8)]
    pub concurrency: usize,

    /// OpenAI 相容介面地址
    #[arg(long, default_value = "https://api.deepseek.com")]
    pub base_url: String,

    /// 模型名
    #[arg(long, default_value = "deepseek-v4-flash")]
    pub model: String,

    /// 金鑰；預設讀環境變數
    #[arg(long, env = "QINGJIAN_API_KEY", hide_env_values = true)]
    pub api_key: String,

    /// 單個請求的超時秒數
    #[arg(long, default_value_t = 180)]
    pub timeout_secs: u64,
}

#[derive(Debug, clap::Args)]
pub struct GenerateArgs {
    /// 一元詞頻表（`詞\t次數`），`dict-convert bigram` 生成的 lm-unigram.tsv
    #[arg(long, default_value = "data/generated/lm-unigram.tsv")]
    pub words: PathBuf,

    /// 只要出現次數不低於這個值的詞
    #[arg(long, default_value_t = 200)]
    pub min_count: u64,

    /// 只要不超過這麼多個字的詞
    #[arg(long, default_value_t = 4)]
    pub max_chars: usize,

    /// 只處理詞頻最高的前 N 個（試跑用）
    #[arg(long)]
    pub limit: Option<usize>,

    /// 結果 JSONL，已有的詞跳過
    #[arg(long, default_value = "data/generated/gloss-llm.jsonl")]
    pub out: PathBuf,

    /// 本次生成哪些學習語言，逗號分隔（en / ja / vi）；預設只生成英日，越南語要顯式傳 `--languages vi`
    #[arg(long, value_delimiter = ',', default_value = "en,ja")]
    pub languages: Vec<String>,

    /// 每個請求帶幾個詞
    #[arg(long, default_value_t = 40)]
    pub batch: usize,

    /// 同時幾個請求
    #[arg(long, default_value_t = 8)]
    pub concurrency: usize,

    /// OpenAI 相容介面地址
    #[arg(long, default_value = "https://api.deepseek.com")]
    pub base_url: String,

    /// 模型名
    #[arg(long, default_value = "deepseek-v4-flash")]
    pub model: String,

    /// 金鑰；預設讀環境變數
    #[arg(long, env = "QINGJIAN_API_KEY", hide_env_values = true)]
    pub api_key: String,

    /// 單個請求的超時秒數
    #[arg(long, default_value_t = 180)]
    pub timeout_secs: u64,
}

#[derive(Debug, clap::Args)]
pub struct ExportArgs {
    /// `generate` 寫出的 JSONL
    #[arg(long, default_value = "data/generated/gloss-llm.jsonl")]
    pub input: PathBuf,

    /// 輸出目錄，寫 glossary-en.tsv、glossary-ja.tsv 與 glossary-vi.tsv
    #[arg(long, default_value = "data/generated")]
    pub out_dir: PathBuf,
}
