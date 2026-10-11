use qingjian_core::PredictionPolicy;
use serde::{Deserialize, Serialize};

/// 雲聯想配置。預設**關閉**，開啟後游標附近的文本會發往 `base_url`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PredictConfig {
    /// 是否啟用。
    pub enabled: bool,

    /// OpenAI 相容介面地址（不含 `/chat/completions`）。
    pub base_url: String,

    /// 模型名。
    pub model: String,

    /// 金鑰。留空則讀 `api_key_env` 指定的環境變數。
    pub api_key: Option<String>,

    /// 存放金鑰的環境變數名。
    pub api_key_env: String,

    /// 單次請求超時（毫秒），超時即丟。
    pub timeout_ms: u64,

    /// 防抖：停止敲鍵多久之後才真正發請求（毫秒）。
    pub debounce_ms: u64,

    /// 游標前最多發多少個字元。
    pub lookback: usize,

    /// 游標後最多發多少個字元。
    pub lookahead: usize,

    /// 雲端詞最多補進候選視窗第一頁末尾幾格；0 表示不要雲端詞，只要整句補全。
    pub slots: usize,

    /// 組句中要不要整句補全。
    pub sentence: bool,

    /// 推理強度，隨請求發 `reasoning_effort`：`none` 關掉模型的思考（聯想要的是快，不是想），
    /// 其餘 minimal / low / medium / high / xhigh 照傳；留空則不發（給不認這個引數的介面）。
    /// DeepSeek V4 這類預設帶思考的模型不關會把 token 預算全花在思考上，正文為空。
    pub reasoning_effort: String,
}

impl Default for PredictConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "https://api.deepseek.com".to_owned(),
            model: "deepseek-v4-flash".to_owned(),
            api_key: None,
            api_key_env: "QINGJIAN_API_KEY".to_owned(),
            timeout_ms: 5000,
            debounce_ms: 300,
            lookback: 64,
            lookahead: 32,
            slots: 2,
            sentence: true,
            reasoning_effort: "none".to_owned(),
        }
    }
}

impl PredictConfig {
    /// 配置裡的金鑰優先，其次環境變數；兩邊都沒有返回 `None`。
    pub fn resolve_api_key(&self) -> Option<String> {
        self.api_key
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(str::to_owned)
            .or_else(|| std::env::var(&self.api_key_env).ok())
            .filter(|k| !k.trim().is_empty())
    }

    pub fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            before: self.lookback,
            after: self.lookahead,
            slots: self.slots,
            // 比槽位多要兩條，與本地候選重複的去掉後還能填滿；問字模式的答案也按這個數要
            max_items: self.slots.max(1) + 2,
            sentence: self.sentence,
        }
    }
}
