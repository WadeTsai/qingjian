//! 隨包資源（詞庫 / 語言模型 / 釋義表 / 等級表 / emoji / 樣例）的定位。
//!
//! 兩套佈局，裝機優先、回落開發：
//! - **裝機**：資源與執行檔同級（安裝程式把 `data\` / `assets\` 裝在 exe 旁）。
//! - **開發**：倉庫 `ime/` 目錄，exe 在 `ime\target\{debug,release}\` 下，往上三層。
//!
//! 相對寫法兩套佈局一致（如 `data/generated/dict.qj`、`assets/levels`），只有根不同。
//! Windows 的 Server 程序與設定視窗共用；macOS 有自己的 `paths.rs`，不走這裡。

use std::path::{Path, PathBuf};

/// 隨包資源的根目錄：其下有 `data/` 與 `assets/`。裝機佈局與 exe 同級，否則回落開發佈局的倉庫根；兩處都沒有為 `None`。
pub fn bundled_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    if has_resources(exe_dir) {
        return Some(exe_dir.to_path_buf());
    }
    // exe → {debug,release} → target → ime
    let dev_root = exe.ancestors().nth(3)?;
    has_resources(dev_root).then(|| dev_root.to_path_buf())
}

/// 一個隨包資源的完整路徑（相對隨包根，如 `data/generated/dict.qj`）；根找不到或該路徑不存在為 `None`。
pub fn bundled_resource(rel: &str) -> Option<PathBuf> {
    let path = bundled_root()?.join(rel);
    path.exists().then_some(path)
}

/// 一個目錄是不是隨包根：有 `data` 或 `assets` 子目錄就算。
fn has_resources(dir: &Path) -> bool {
    dir.join("data").is_dir() || dir.join("assets").is_dir()
}
