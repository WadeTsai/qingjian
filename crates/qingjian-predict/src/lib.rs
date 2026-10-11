//! 雲聯想：[`qingjian_core::Predictor`] 的網路實現，接 OpenAI 相容的聊天介面（首個目標是 DeepSeek）。
//!
//! 請求在獨立執行緒裡發，主執行緒只往通道里丟請求、從通道里取結果，永遠不阻塞輸入。
//! 「最新請求優先」：防抖視窗內只發最後一個請求；結果帶序號，Core 丟棄過期的。
//!
//! [`CloudGlossFiller`] 是釋義兜底（[`qingjian_core::GlossFiller`]）的網路實現，獨立執行緒，攢批問、不丟請求。

mod cache;
mod chat_client;
mod cloud_predictor;
mod config;
mod connection;
mod error;
mod gloss;
mod prompt;
mod worker;

pub use cloud_predictor::CloudPredictor;
pub use config::PredictConfig;
pub use connection::{ConnectionReport, ConnectionTest};
pub use error::PredictError;
pub use gloss::CloudGlossFiller;
