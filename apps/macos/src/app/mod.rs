//! 程序級基礎設施：bundle 資訊、日誌、資料路徑、配置檔案的執行時狀態。與 IMK、視窗都無關。

pub mod bundle;
pub mod input_source;
pub mod logging;
pub mod paths;
mod settings;

pub use bundle::BundleInfo;
pub use settings::Settings;
