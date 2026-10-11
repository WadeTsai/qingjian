//! 輸入法圖示：`AddLanguageProfile` 要一個圖示檔案路徑。不嵌 DLL 資源（要 rc.exe / windres 參與構建，
//! 本機交叉 check 做不了），而是編譯進來、註冊時寫成機器級 `.ico`（regsvr32 本來就要管理員）。

use std::path::PathBuf;

/// 與 macOS 端同一張 logo 生成的多尺寸 `.ico`。
const ICON: &[u8] = include_bytes!("../../../resources/qingjian.ico");

/// `%ProgramData%\Qingjian\qingjian.ico`。註冊是機器級的，別的使用者也要能讀到，所以不能放使用者目錄。
fn path() -> Option<PathBuf> {
    std::env::var_os("ProgramData")
        .map(|base| PathBuf::from(base).join("Qingjian").join("qingjian.ico"))
}

/// 寫不了返回 `None`，呼叫方註冊成無圖示。
pub(super) fn install() -> Option<PathBuf> {
    let path = path()?;
    let dir = path.parent()?;
    if let Err(error) = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&path, ICON)) {
        crate::com::log::log(&format!("寫圖示檔案失敗，註冊成無圖示: {error}"));
        return None;
    }
    Some(path)
}

pub(super) fn uninstall() {
    if let Some(path) = path() {
        let _ = std::fs::remove_file(path);
    }
}
