//! DLL 的「引擎層」：不跑 Engine，而是連獨立 Server 程序，把按鍵 / 上屏編排成協議訊息。
//! [`EngineClient`] 泛型在任意雙工位元組流上，可脫離 Windows 端到端測；`cfg(windows)` 的 [`pipe`] 負責連管道。

mod engine;
mod reply;
mod response;

#[cfg(windows)]
pub mod pipe;

pub use engine::EngineClient;
pub use reply::KeyReply;
pub use response::{KeyResponse, ModeSyncReply};
