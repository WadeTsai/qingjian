//! 檢查更新：讀官網的版本索引 `releases.json`，驗 ed25519 簽名，按平臺、CPU 與渠道挑出比當前新的版本。
//!
//! 只提示不安裝。殼在自己的定時器裡調 [`Checker::poll`]，到點了它起一個一次性執行緒去查，結果寫進資料目錄的 `update.json`；
//! 選單與設定頁用 [`Checker::available`] 取要提示的版本。設計與發版側的簽名見 `docs/design/update.md`。

mod checker;
mod error;
mod index;
mod target;
mod version;

pub use checker::{Available, Checker, UpdateState};
pub use error::UpdateError;
pub use index::{Asset, Index, PUBLIC_KEYS, Release, SCHEMA_VERSION, verify};
pub use target::Target;
pub use version::Version;

/// 下載頁，提示裡點開的就是它。
pub const DOWNLOAD_URL: &str = "https://qingjian.app/download";
