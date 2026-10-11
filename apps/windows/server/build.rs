//! 嵌 uiAccess manifest：候選視窗要蓋過商店 / 工作列搜尋這些高 z-band 宿主，`SetWindowPos(HWND_TOPMOST)`
//! 才能升進 UIAccess 高帶。系統只對簽名且裝在 Program Files 的 exe 授予，光有 manifest 不夠；
//! **沒簽名的 exe 帶 uiAccess=true 會直接起不來**（「從伺服器返回了一個參照」），所以預設不帶，簽過名的構建才設
//! `QINGJIAN_UIACCESS=1` 開啟（`build.ps1 -Sign`）；不帶的代價是候選窗在 UWP 宿主裡可能被蓋住（使用者文件已列為已知問題）。
//! manifest 預設含 PerMonitorV2 DPI 感知，與執行時那次 `SetProcessDpiAwarenessContext` 一致。
//! 另把青簡圖示嵌進 exe（工作管理員 / 啟動項裡顯示）。

use embed_manifest::manifest::ExecutionLevel;
use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    // build.rs 跑在宿主機上，只有目標是 Windows 時才嵌。
    println!("cargo:rerun-if-env-changed=QINGJIAN_UIACCESS");
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let ui_access = std::env::var("QINGJIAN_UIACCESS").is_ok_and(|v| v == "1");
        if ui_access {
            println!("cargo:warning=QINGJIAN_UIACCESS=1：Server 带 uiAccess，没签名的话起不来");
        }
        let manifest = new_manifest("Qingjian.Server")
            .requested_execution_level(ExecutionLevel::AsInvoker)
            .ui_access(ui_access);
        embed_manifest(manifest).expect("嵌入 Server manifest 失败");
    }
    embed_icon();
    println!("cargo:rerun-if-changed=build.rs");
}

/// 圖示資源要 `rc.exe`（MSVC）編，只在 Windows 宿主上做；失敗只警告，別讓編譯掛掉。
/// winresource 預設不帶 manifest，與上面連結器嵌的那份不衝突。
#[cfg(windows)]
fn embed_icon() {
    const ICON: &str = "../tsf/resources/qingjian.ico";
    println!("cargo:rerun-if-changed={ICON}");
    if let Err(error) = winresource::WindowsResource::new().set_icon(ICON).compile() {
        println!("cargo:warning=嵌入 Server 图标失败: {error}");
    }
}

#[cfg(not(windows))]
fn embed_icon() {}
