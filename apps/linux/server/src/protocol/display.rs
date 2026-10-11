//! 視窗提交後回報實際完整可見的譯詞，索引必須屬於當前幀。
use super::DisplayIdentity;
use qingjian_platform::protocol::SessionId;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayAcknowledged {
    /// 連線內會話編號，由 IPC 層重對映。
    pub session: SessionId,

    /// 當前展示身份。
    pub identity: DisplayIdentity,

    /// (候選槽位, Translation 義項編號)，不傳候選文字。
    pub senses: Vec<(usize, usize)>,
}
