//! 各程序日誌的公共部分：按天分檔案的命名與清理（[`daily`]）、金鑰掩碼（[`secrets`]）。
//! 日誌目錄本身在 [`crate::dirs::log_dir`]。

pub mod daily;
pub mod secrets;
