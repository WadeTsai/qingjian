//! Server 的庫部分：Engine 裝配（[`assembly`]）、協議分派（[`Router`]）與傳輸（[`ipc`]），供 bin 與整合測試共用。

pub mod assembly;
pub mod dispatch;
pub mod error;
pub mod ipc;
/// 候選視窗 / 狀態條的自繪執行緒；僅 Windows。
#[cfg(windows)]
pub mod ui;

pub use assembly::{AssemblySpec, LanguageModelFiles};
pub use dispatch::{Router, RouterConfig};
pub use error::ServerError;
