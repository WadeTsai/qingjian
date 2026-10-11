mod channel;

pub use channel::UpdateChannel;

use serde::{Deserialize, Serialize};

/// 配置檔案 `[update]` 分節：檢查更新。
///
/// 每天向官網讀一次版本索引，有新版就在選單與設定的「關於」頁提示；請求不帶任何標識，不下載也不安裝。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    /// 開著就每天檢查一次。
    pub check: bool,

    /// 看哪個渠道的版本。
    pub channel: UpdateChannel,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            check: true,
            channel: UpdateChannel::default(),
        }
    }
}
