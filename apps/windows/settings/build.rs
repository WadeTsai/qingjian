//! 編譯時抓 git 構建標識（分支@短雜湊 (日期)，工作區有改動短雜湊後加 +，與 macOS 的 bundle.sh 一致）塞進 `QINGJIAN_BUILD`，「關於」頁顯示；拿不到就不設。
//! 版本號是 `-dev` 時再接短雜湊塞進 `QINGJIAN_VERSION`（與安裝包的版本一致，見 installer\build.ps1）。
//! 在 Windows 上編時把青簡圖示嵌進 exe（開始選單 / 工作列 / 搜尋裡顯示的就是它）。

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    if let Some(build) = git_build() {
        println!("cargo:rustc-env=QINGJIAN_BUILD={build}");
    }
    println!("cargo:rustc-env=QINGJIAN_VERSION={}", dev_version());
    embed_icon();
    stage_windows_runtime();
}

/// 圖示資源要 `rc.exe`（MSVC）編，只在 Windows 宿主上做；失敗只警告，別讓編譯掛掉。
#[cfg(windows)]
fn embed_icon() {
    const ICON: &str = "../tsf/resources/qingjian.ico";
    println!("cargo:rerun-if-changed={ICON}");
    if let Err(error) = winresource::WindowsResource::new().set_icon(ICON).compile() {
        println!("cargo:warning=嵌入设置程序图标失败: {error}");
    }
}

#[cfg(not(windows))]
fn embed_icon() {}

/// 自包含部署 Windows App Runtime，並讓 exe 在 Windows 10 上也能載入（定位與取捨見 docs\notes\windows-win10.md）。
///
/// 兩件事都得做：
/// 1. **自帶執行時**：Reactor 的框架依賴引導調的是 Windows 11 才有的 AppModel API
///    （`TryCreatePackageDependency` / `AddPackageDependency`，`api-ms-win-appmodel-runtime-l1-1-5.dll`），
///    Windows 10 上沒有這兩個函式，也就沒法把機器上裝的框架包加進程序包圖；改成自包含部署後 UI 只用
///    exe 旁邊這份執行時，`windows-reactor-setup` 負責鋪檔案並按自包含標記嵌清單。
/// 2. **延遲載入那兩個 API**：它們是早期繫結，函式名會進 exe 的匯入表（IAT），Windows 10 在**載入期**就報
///    「無法定位程式輸入點 TryCreatePackageDependency」，根本進不到自包含分支。延遲載入後 IAT 裡沒有它們，
///    Windows 10 能正常啟動；而自包含部署下這兩個函式不會被呼叫。
#[cfg(windows)]
fn stage_windows_runtime() {
    // 只對 MSVC 目標做：`as_self_contained` 不支援 gnu 目標，`/DELAYLOAD` 與 `delayimp.lib` 也是 MSVC 連結器的；
    // Windows 宿主交叉編 windows-gnu 只是本機檢查用，不發版。
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    windows_reactor_setup::as_self_contained();
    println!("cargo:rustc-link-arg-bins=/DELAYLOAD:api-ms-win-appmodel-runtime-l1-1-5.dll");
    println!("cargo:rustc-link-arg-bins=delayimp.lib");
}

#[cfg(not(windows))]
fn stage_windows_runtime() {}

/// `CARGO_PKG_VERSION`，`-dev` 結尾時接 `-<短雜湊>`（髒加 +）。
fn dev_version() -> String {
    let version = env!("CARGO_PKG_VERSION").to_owned();
    if !version.ends_with("-dev") {
        return version;
    }
    let Some(mut hash) = git(&["rev-parse", "--short", "HEAD"]) else {
        return version;
    };
    if git(&["status", "--porcelain"]).is_some() {
        hash.push('+');
    }
    format!("{version}-{hash}")
}

fn git_build() -> Option<String> {
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    let mut hash = git(&["rev-parse", "--short", "HEAD"])?;
    if git(&["status", "--porcelain"]).is_some() {
        hash.push('+');
    }
    let date = git(&[
        "show",
        "-s",
        "--format=%cd",
        "--date=format:%Y-%m-%d",
        "HEAD",
    ])?;
    Some(format!("{branch}@{hash} ({date})"))
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!text.is_empty()).then_some(text)
}
