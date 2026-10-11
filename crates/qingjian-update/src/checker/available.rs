use serde::{Deserialize, Serialize};

/// 查到的新版本，夠選單和「關於」頁顯示用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Available {
    pub version: String,

    /// 索引裡的渠道：`alpha` / `beta` / `rc` / `stable`。
    pub channel: String,

    pub date: String,

    /// 更新日誌，一行一條。
    pub notes: Vec<String>,
}
