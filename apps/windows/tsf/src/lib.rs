//! 青簡 Windows 輸入法的 TSF 文本服務 DLL。
//!
//! TSF 會把這個 DLL 載入進每一個應用程序，所以 Engine 不在這裡（在獨立的 Server 程序）。
//! DLL 只把 TSF 的按鍵翻成協議訊息發給 Server、把回的文本 / 拼音行寫進文件；候選視窗由 Server 自繪。
//!
//! - [`client`]：引擎層，連 Server 的管道客戶端，平臺無關部分可脫離 Windows 端到端測。
//! - `com`（`cfg(windows)`）：COM 入口、TSF 介面實現、編輯會話 / 組句、雲聯想輪詢、自注冊。

pub mod client;
#[cfg(windows)]
mod com;
pub mod error;

pub use error::ClientError;
