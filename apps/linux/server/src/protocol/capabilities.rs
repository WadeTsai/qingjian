//! 框架能力事實；輸入策略由 Server 決定。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub sensitive: bool,

    pub password: bool,

    pub disabled: bool,
}
