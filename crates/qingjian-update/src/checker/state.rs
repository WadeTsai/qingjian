use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use qingjian_platform::UpdateChannel;

use super::Available;

/// 上次檢查的結果，存在資料目錄的 `update.json`；Windows 的設定程式直接讀這個檔案。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateState {
    /// 上次查成功的時間（Unix 秒）；0 = 還沒查過。
    pub checked_at: u64,

    /// 那次查的是哪個渠道；換了渠道要立刻重查。
    pub channel: Option<UpdateChannel>,

    /// 那次查到的新版本。
    pub available: Option<Available>,
}

impl UpdateState {
    /// 讀不到或讀不懂都當沒查過。
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// 先寫臨時檔案再改名，讀的一方不會看到半截。
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&temporary, path)
    }

    pub(crate) fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
    }
}
