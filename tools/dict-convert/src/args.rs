use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::mmh::MmhReferenceOptions;
use crate::stroke::StrokeOptions;

#[derive(Debug, Parser)]
#[command(
    name = "qingjian-dict-convert",
    about = "把第三方词库 / 词典转换成青简的 TSV，或把 TSV 打包成 .qj"
)]
pub struct Args {
    /// 輸出目錄
    #[arg(long, default_value = "data/generated")]
    pub out_dir: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 青簡基礎詞庫：從「輸入法字詞庫_分類整理版」資料包 + Unihan 讀音建 dict.tsv（兩遍跑，見模組文件）
    Lexicon {
        /// 資料包目錄（含 01_characters / 02_common / 03_domains），隨倉庫放在 assets/lexicon
        #[arg(long, default_value = "assets/lexicon")]
        pack: PathBuf,

        /// Unihan_Readings.txt
        #[arg(long, default_value = "data/unihan/Unihan_Readings.txt")]
        unihan: PathBuf,

        /// LLM 標註的多音字詞讀音（`gloss-gen pinyin` 的 JSONL）
        #[arg(long)]
        pinyin: Option<PathBuf>,

        /// 語料詞頻（lm-unigram.tsv）；沒有就按排序號 / 文件頻次給底值
        #[arg(long)]
        frequency: Option<PathBuf>,

        /// 把仍靠猜讀音的多音字詞寫到這個檔案（一行一個），交給 `gloss-gen pinyin`
        #[arg(long)]
        emit_ambiguous: Option<PathBuf>,

        /// 額外併入的詞（`詞\t次數`，`mine` 挖出來的 oov-candidates.tsv，可給多個）：詞庫裡沒有的按次數當詞頻加進去，讀音同領域詞
        #[arg(long)]
        extra_words: Vec<PathBuf>,

        /// 領域詞在語料裡出現不少於這個次數就留在基礎詞庫，否則拆到 dicts/<領域>.qj
        #[arg(long, default_value_t = 50)]
        domain_keep_min: u64,
    },

    /// 形碼碼錶（五筆）：Rime `.dict.yaml` → `詞\t編碼\t詞頻`。詞頻由青簡詞庫按詞面回填，不用碼錶自帶的權重
    Wubi {
        /// 輸入的 Rime 碼錶（`.dict.yaml`，如極點 86 五筆）
        input: PathBuf,

        /// 詞頻來源：青簡詞庫 TSV（`詞\t拼音\t詞頻`）。可給多個（基礎詞庫 + 隨包領域詞庫），同一個詞取詞頻最大的那份
        #[arg(long, default_value = "assets/lexicon/dict.tsv", num_args = 1..)]
        frequency: Vec<PathBuf>,

        /// 輸出檔名（寫在 --out-dir 下）
        #[arg(long, default_value = "wubi86.tsv")]
        name: String,
    },

    /// CC-CEDICT `cedict_ts.u8` → glossary-en.tsv
    Cedict {
        /// 輸入檔案
        input: PathBuf,
    },

    /// 英文詞表（每行 `詞\t編碼[\t…]`，帶不帶表頭都行，比如 `assets/lexicon/05_english/00_all_words.tsv`）→ english.tsv
    English {
        /// 輸入檔案
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// 詞頻表（`編碼\t詞頻`，`tools/corpus/english_frequency.py` 生成）；給了就寫進第三列，字首補全按它排
        #[arg(long)]
        frequency: Option<PathBuf>,
    },

    /// Unicode CLDR emoji annotations（`annotations/<語言>/annotations.json`、`annotationsDerived/…`）→ emoji-<語言>.tsv：`詞\temoji …`
    Emoji {
        /// 輸入的 JSON 檔案
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// 語言程式碼，決定輸出檔名（zh → emoji-zh.tsv，en → emoji-en.tsv）
        #[arg(long, default_value = "zh")]
        language: String,
    },

    /// 純文本語料（每行一段）→ lm-unigram.tsv + lm-bigram.tsv：按詞庫分詞後統計詞級一元 / 二元計數
    Bigram {
        /// 語料檔案（UTF-8 純文本，簡體）
        #[arg(required = true)]
        corpus: Vec<PathBuf>,

        /// 分詞用的詞庫（青簡 TSV）；同目錄 dicts/ 下的領域詞庫會一併用於分詞（詞表與拆分前一致）
        #[arg(long, default_value = "data/generated/dict.tsv")]
        dict: PathBuf,

        /// 短語層檔案（assets/lexicon/phrases.tsv，可給多個，人工挑的領域詞 domain_words.tsv 也走這條路）：裡面的詞不參與分詞，統計完按成分合成它們的一元 / 二元計數（見 bigram.rs 模組註釋）
        #[arg(long)]
        phrases: Vec<PathBuf>,

        /// 品牌詞檔案（assets/lexicon/brand.tsv，可給多個，中英混雜詞 mixed_words.tsv 也走這條路）：語料裡沒有的詞按檔案給的次數寫進一元表。
        /// 與 --phrases 的區別：合成計數要成分詞在語料裡，C盤 的 C 不是語料 token，只能直接給
        #[arg(long)]
        brand: Vec<PathBuf>,

        /// 計數低於此值的二元組不輸出
        #[arg(long, default_value_t = 3)]
        min_count: u32,

        /// 最多輸出多少條二元組（按計數取前 N）
        #[arg(long, default_value_t = 3_000_000)]
        max_bigrams: usize,
    },

    /// 從語料裡挖詞庫沒收的詞：分詞時被拆成連續單字的段按子串計數，出現夠多的寫到 oov-candidates.tsv（再交給 gloss-gen pinyin 標音、lexicon --extra-words 併入）
    Mine {
        /// 語料檔案（UTF-8 純文本，簡體）；給了 --candidates 就不用掃語料
        #[arg(required_unless_present = "candidates")]
        corpus: Vec<PathBuf>,

        /// 語言模型一元表（`詞\t次數`），算相鄰字對 PMI 用
        #[arg(long, default_value = "data/generated/lm-unigram.tsv")]
        frequency: PathBuf,

        /// 相鄰字對 PMI 的下限；0 不過濾
        #[arg(long, default_value_t = 3.0)]
        min_pmi: f64,

        /// 跳過掃語料，直接過濾上一次寫出的 oov-candidates.tsv（調閾值用）
        #[arg(long)]
        candidates: Option<PathBuf>,

        /// 分詞用的詞庫（青簡 TSV）
        #[arg(long, default_value = "assets/lexicon/dict.tsv")]
        dict: PathBuf,

        /// 出現次數低於此值的不要
        #[arg(long, default_value_t = 200)]
        min_count: u32,

        /// 最多幾個字
        #[arg(long, default_value_t = 4)]
        max_chars: usize,
    },

    /// 短語層：從 bigram 表的相鄰兩詞與語料的相鄰三詞裡挖 我的 / 不知道 這類人會整塊打的組合 → phrases.tsv（人工過一遍後拷進 assets/lexicon/，`lexicon --extra-words` 併入）
    Phrases {
        /// 語料檔案
        #[arg(required = true)]
        corpus: Vec<PathBuf>,

        /// 對話語料（corpus 裡的一個）：短語在它裡面的次數也要夠 min_count，維基模板句進不來
        #[arg(long, default_value = "data/corpus/lccc.txt")]
        dialogue: PathBuf,

        /// 分詞與成分讀音用的詞庫（青簡 TSV，同目錄 dicts/ 一併讀）
        #[arg(long, default_value = "data/generated/dict.tsv")]
        dict: PathBuf,

        /// 上一次的 phrases.tsv：詞庫已併入短語時重跑要給，裡面的詞先從分詞詞表裡摘掉（否則 我的 是一個詞，挖不出 我 + 的）
        #[arg(long)]
        refresh: Option<PathBuf>,

        /// 次數下限
        #[arg(long, default_value_t = 2000)]
        min_count: u32,

        /// 最多幾個字
        #[arg(long, default_value_t = 4)]
        max_chars: usize,
    },

    /// 筆畫表：CNS11643 全字型檔「筆順資料」+ 大陸序覆蓋表 → `codes/stroke.tsv`（隨包筆畫碼錶的源資料，見模組文件）。
    /// 引數的 clap 定義在 `stroke::StrokeOptions`，加引數只動那一處
    Stroke(StrokeOptions),

    /// 筆畫對照表：從 hanzi-writer-data（Make Me a Hanzi；Arphic 許可，不進倉庫）生成 `stroke --verify`
    /// 用的兩張開發期對照表到 `data/mmh/`（筆畫數、首筆幾何類別，見模組文件與 assets/stroke/README.md）
    MmhReference(MmhReferenceOptions),

    /// 把 TSV 打包成 `.qj` 容器（mmap 直接用，啟動近零耗時）：`dict` 讀 dict.tsv 寫 dict.qj，`lm` 讀 lm-unigram/bigram.tsv 寫 lm.qj，
    /// `glossary --language en` 讀 glossary-en.tsv 寫 glossary-en.qj；`model` 把匯出的三件套目錄（預設 data/models/hanzhang-zhiwei）
    /// 打成一個 .qjm（`--out-dir data/models/hanzhang-zhiwei` 就寫回原目錄，隨包只帶這一個檔案）；
    /// `codes` 是唯一不「原樣落盤」的一種：讀筆畫表與詞庫，按取碼規則算成本地碼錶 codes/stroke.qj（見 codes 模組）
    Pack {
        /// 打包哪種資料
        kind: PackKind,

        /// 輸入檔案；`dict` 一個 TSV，`lm` 兩個（一元表、二元表），`model` 一個目錄。預設從輸出目錄裡找同名 TSV（`model` 預設 data/models/hanzhang-zhiwei）
        #[arg(long, num_args = 1..)]
        input: Vec<PathBuf>,

        /// `codes` 用：筆畫表（`stroke` 子命令的產物，`字\t序列`）；預設 <輸出目錄>/codes/stroke.tsv
        #[arg(long)]
        stroke: Option<PathBuf>,

        /// `codes` 用：取碼用的詞庫（`.qj` 或 TSV）；預設 <輸出目錄>/dict.qj
        #[arg(long)]
        dict: Option<PathBuf>,

        /// `codes` 或 `model` 用：輸出檔案；模型預設 <輸出目錄>/model.qjm
        #[arg(long)]
        output: Option<PathBuf>,

        /// 後設資料：名稱（`codes` 預設「筆畫」，別的種類必填）
        #[arg(long, default_value = "")]
        name: String,

        /// 後設資料：許可證（SPDX 標識，如 GPL-3.0-only、CC-BY-SA-4.0）
        #[arg(long, default_value = "")]
        license: String,

        /// 後設資料：署名 / 版權行
        #[arg(long, default_value = "")]
        attribution: String,

        /// 後設資料：來源 URL
        #[arg(long, default_value = "")]
        source: String,

        /// 後設資料：資料版本（上游版本號或日期）
        #[arg(long, default_value = "")]
        data_version: String,

        /// `glossary` 專用：釋義表的語言程式碼（en / ja / zh / es / vi），決定輸出檔名 glossary-<語言>.qj
        #[arg(long, default_value = "en")]
        language: String,
    },
}

/// `pack` 能打的資料種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PackKind {
    /// 拼音詞庫
    Dict,

    /// 詞級 bigram 語言模型
    Lm,

    /// 釋義表（glossary-<語言>.tsv → glossary-<語言>.qj）
    Glossary,

    /// 本地整句模型（三件套目錄 → .qjm）
    Model,

    /// 筆畫碼錶（筆畫表 + 詞庫 → codes/stroke.qj，隨包原生碼錶）
    Codes,
}

impl PackKind {
    /// 子命令裡寫的名字（報錯文案用）。
    pub fn name(self) -> &'static str {
        match self {
            Self::Dict => "dict",
            Self::Lm => "lm",
            Self::Glossary => "glossary",
            Self::Model => "model",
            Self::Codes => "codes",
        }
    }
}
