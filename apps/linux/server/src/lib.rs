//! Linux Server：持有 Core Engine，通過 Unix domain socket 服務多個 Fcitx5 會話。
pub mod assembly;
pub mod dispatch;
pub mod error;
#[cfg(target_os = "linux")]
pub mod ipc;
pub mod protocol;
pub use assembly::{AssemblySpec, LanguageModelFiles};
pub use dispatch::{Router, RouterConfig, find_model};
pub use error::ServerError;
