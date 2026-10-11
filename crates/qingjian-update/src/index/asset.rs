use serde::Deserialize;

/// 一個安裝包：只看它是給哪個平臺、哪種 CPU 的。
#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub platform: String,

    /// `arm64` / `x86_64`；0.1.3 之前的索引沒有這個欄位。
    #[serde(default)]
    pub cpu: String,
}
