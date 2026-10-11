use std::path::PathBuf;

use clap::Parser;

/// 按優先順序挑一個存在的資料檔案：`data/generated/` 裡打包好的 `.qj`、那裡的 TSV、倉庫自帶的產品資料
/// （`assets/lexicon/dict.tsv`、`assets/glossary/glossary-*.tsv`、`assets/lexicon/english.tsv`），最後是 `assets/sample/` 的樣例。
pub fn default_data_file(name: &str) -> PathBuf {
    let generated = PathBuf::from("data/generated").join(name);
    let packed = generated.with_extension("qj");
    let shipped = if name.starts_with("glossary-") {
        PathBuf::from("assets/glossary").join(name)
    } else {
        PathBuf::from("assets/lexicon").join(name)
    };
    for candidate in [packed, generated, shipped] {
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("assets/sample").join(name)
}

/// 預設配置檔案位置：與輸入法共用同一份。
pub fn default_config_file() -> PathBuf {
    if cfg!(target_os = "macos")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join("Library/Application Support/Qingjian/config.toml");
    }
    PathBuf::from("config.toml")
}

#[derive(Debug, Parser)]
#[command(name = "qingjian", about = "青简输入法 Core 测试工具")]
pub struct Args {
    /// 詞庫路徑（TSV）。預設：data/generated/dict.tsv 存在就用它，否則 assets/sample/dict.tsv
    #[arg(long)]
    pub dict: Option<PathBuf>,

    /// 釋義表路徑。預設：data/generated/glossary-<language>.tsv 存在就用它，否則 assets/sample/ 下的同名檔案
    #[arg(long)]
    pub glossary: Option<PathBuf>,

    /// 學習語言：en / ja / es / vi。也可用環境變數 QINGJIAN_LEARNING_LANGUAGE
    #[arg(long, env = "QINGJIAN_LEARNING_LANGUAGE", default_value = "en")]
    pub language: String,

    /// 附加詞庫（.qj 或 TSV），可給多個，與主詞庫一起查
    #[arg(long)]
    pub extra_dict: Vec<PathBuf>,

    /// 輔碼碼錶（.qj，或 `詞<TAB>碼` 的 TSV），可給多個一起篩。給了之後 `kaifa;kf` 這樣的輸入
    /// 按輔碼態走：觸發鍵進輔碼態、之後的字母按碼縮小候選
    #[arg(long)]
    pub aux_table: Vec<PathBuf>,

    /// 查碼：列印這些詞在已裝碼錶裡的全部碼（配 --aux-table 用），逗號分隔或多次給；查完即退出
    #[arg(long, value_delimiter = ',')]
    pub aux_query: Vec<String>,

    /// 英文詞表路徑（中英混輸）。預設：data/generated/english.tsv 存在就用它，否則不啟用
    #[arg(long)]
    pub english: Option<PathBuf>,

    /// 使用者詞頻檔案；給了就在退出時寫回，不給則只在本次會話內學習
    #[arg(long)]
    pub user_dict: Option<PathBuf>,

    /// 配置檔案路徑。預設：~/Library/Application Support/Qingjian/config.toml（macOS）或 ./config.toml
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// 啟用雲聯想（無視配置裡的 enabled）；金鑰來自配置或 QINGJIAN_API_KEY（`api_key_env`）
    #[arg(long)]
    pub predict: bool,

    /// 模糊音，逗號分隔（z-zh,c-ch,s-sh,n-l,f-h,l-r,an-ang,en-eng,in-ing），`all` 全開；給了就覆蓋配置裡的 [fuzzy]
    #[arg(long, value_delimiter = ',')]
    pub fuzzy: Vec<String>,

    /// 英文模式（輸入法裡是 Caps Lock 亮著）：字母不當拼音，候選來自英文詞表的補全與拼錯糾正
    #[arg(long)]
    pub english_mode: bool,

    /// 開啟中文優先（配置 [general] chinese_first = true）：整段是英文詞時中文候選排第一、英文第二，評測兩種排法用
    #[arg(long)]
    pub chinese_first: bool,

    /// 雙拼方案（xiaohe / ziranma / microsoft / sogou / abc / xiaolang / shoudao），覆蓋配置裡的 [general] shuangpin；off 強制全拼
    #[arg(long)]
    pub shuangpin: Option<String>,

    /// 形碼碼錶（五筆）的 TSV 檔案（`詞\t編碼\t詞頻`）：給了就用編碼查表，不走拼音那一套
    #[arg(long, value_name = "码表")]
    pub wubi: Option<PathBuf>,

    /// 神經重打分：字級 Transformer 的 .qjm 檔案或匯出目錄（model.safetensors / config.json / vocab.json），整句前幾條路徑用它重排
    #[arg(long)]
    pub neural: Option<PathBuf>,

    /// 神經重打分的權重 λ（0 到 1，預設 0.5）：最終分 = 路徑分 + λ·(神經分 − 靜態二元分)，個人學習與代價不受影響
    #[arg(long)]
    pub neural_weight: Option<f64>,

    /// 神經重打分的門檻（nat，預設不設）：路徑分落後最優路徑超過這麼多的不參與重排
    #[arg(long)]
    pub neural_margin: Option<f64>,

    /// 神經重打分給模型看的前文字元數（預設 64，0 為不給前文）
    #[arg(long)]
    pub neural_context: Option<usize>,

    /// 神經重打分走後臺執行緒（輸入法殼裡的接法）：查詢先按詞級模型出候選，再請求 / 等待重打分後重查一次；結果應與同步一致
    #[arg(long)]
    pub neural_async: bool,

    /// 逐鍵模式：把每個輸入當作一鍵一鍵敲進去，每個字首都查一次，列印每鍵各階段耗時（效能測試用）
    #[arg(long)]
    pub typing: bool,

    /// 只顯示前 N 個候選
    #[arg(long, default_value_t = 9)]
    pub limit: usize,

    /// 回放評測：讀輸入日誌（input-log.jsonl），把每次上屏時的鍵重新餵給引擎，算首選命中率等指標。只在記憶體裡學習，不寫任何檔案
    #[arg(long)]
    pub replay: Option<PathBuf>,

    /// 回放 / 整句評測時列印前 N 條沒命中首選的例子
    #[arg(long, default_value_t = 20)]
    pub misses: usize,

    /// 覆蓋引擎裡的調參常數，`名=值`，逗號分隔或多次給。名字：lambda / k / cap / discount（個人 n-gram 插值 λ / K / 封頂 / 三元折扣），
    /// transpose / substitute / extra / missing / typo-cap / correction（敲錯四類代價 / 個人折扣上限 / 整段糾錯代價）
    #[arg(long, value_delimiter = ',')]
    pub tune: Vec<String>,

    /// 整句評測：讀中文文本（一行一段，按標點切句、按詞庫轉成全拼）或 `--eval-save` 凍結下來的三列檔案，
    /// 冷啟動餵給引擎看整句能不能還原原句；可給多個檔案
    #[arg(long, num_args = 1..)]
    pub eval_text: Vec<PathBuf>,

    /// 把整句評測用到的句子集寫成 `句子\t拼音\t上文` 三列檔案，下次直接 `--eval-text` 它，保證比的是同一份句子
    #[arg(long)]
    pub eval_save: Option<PathBuf>,

    /// 實驗專用：P2C 按完整拼音給整句路徑打分；僅用於 --eval-text，不看上文
    #[arg(long, requires = "eval_text", conflicts_with_all = ["neural", "neural_async", "neural_context"])]
    pub eval_p2c: Option<PathBuf>,

    /// 實驗專用：在同一份句子集上讓 P2C 直接生成整句（不經詞圖、不經重排），量重排架構的上限代價
    #[arg(long, requires = "eval_text")]
    pub eval_generate: Option<PathBuf>,

    /// 將整句評測逐條結果寫成 JSONL（含句子、候選和延遲）
    #[arg(long, requires = "eval_text")]
    pub eval_details: Option<PathBuf>,

    /// 冷啟動字詞評測：讀取 JSONL，不載入個人配置或個人學習檔案
    #[arg(long, requires = "cold_output", conflicts_with_all = ["user_dict", "config", "predict", "replay", "eval_text", "english_mode", "shuangpin", "wubi", "aux_table", "tune", "fuzzy", "inputs", "neural_async"])]
    pub eval_cold: Option<PathBuf>,

    /// 冷啟動逐詞結果 JSONL（拒絕覆蓋）
    #[arg(long, requires = "eval_cold")]
    pub cold_output: Option<PathBuf>,

    /// 同時評測 P2C 生成及聯合候選；不改變引擎原候選
    #[arg(long, requires = "eval_cold")]
    pub cold_model: Option<PathBuf>,

    /// 直接查詢這些拼音後退出；不給則進入互動模式
    pub inputs: Vec<String>,
}
