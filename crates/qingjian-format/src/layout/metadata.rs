use serde::{Deserialize, Serialize};

/// 資料檔案的來歷：名稱、許可證、署名、來源。寫在每個 `.qj` 的第一節，偏好設定裡的詞庫列表直接顯示它。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// 給人看的名稱（「霧凇拼音」「青簡基礎詞庫」）。
    pub name: String,

    /// 許可證標識（SPDX 表示式，如 `GPL-3.0-only`、`CC-BY-SA-4.0`）。
    #[serde(default)]
    pub license: String,

    /// 署名 / 版權行。
    #[serde(default)]
    pub attribution: String,

    /// 來源 URL。
    #[serde(default)]
    pub source: String,

    /// 資料版本（上游版本號或日期）。
    #[serde(default)]
    pub version: String,

    /// 條數（詞庫是詞條數，語言模型是 bigram 對數），只用於顯示。
    #[serde(default)]
    pub entries: u64,

    /// 誰生成的（`qingjian-dict-convert 0.1.0`）。
    #[serde(default)]
    pub generator: String,
}

impl Metadata {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }
}
