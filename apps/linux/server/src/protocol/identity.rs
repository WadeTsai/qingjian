//! 連線代次、上下文身份與服務端幀版本共同標識一次展示。
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayIdentity {
    /// 外掛當前連線代次。
    pub generation: u64,

    /// Fcitx InputContext UUID。
    pub context: String,

    /// Server 當前幀版本。
    pub revision: u64,
}
