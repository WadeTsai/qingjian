//! 連不上 Server 時自己拉起它。
//!
//! Server 只在登入時由「啟動」資料夾的快捷方式拉起（Explorer 走 `ShellExecute`，見 `qingjian.iss`），
//! 中途掛了（崩潰 / 被殺 / 裝完沒重啟）以前 DLL 只能靜默吞鍵到下次登入。這裡在連線失敗後起一次與
//! DLL 同目錄的 `qingjian-server.exe`：用 `ShellExecuteW` 而不是 `CreateProcess`——`uiAccess=true`
//! 的 exe 用 `CreateProcess` 拉不起來（報 740），ShellExecute 等同雙擊，兩種構建都行。
//!
//! 兩道閘門防重複啟動：程序內的冷卻時間（同一應用連敲只試一次），與跨程序的命名互斥體
//! （多個應用同時發現 Server 不在，只起一個）。找不到 exe / 起失敗記日誌返回 `false`，呼叫方退回原來的退避重連。
//!
//! 另一道門是宿主的完整性級別：輸入法會被載入進登入介面（LogonUI）、UAC 的 consent.exe 這類
//! 以 SYSTEM 跑在安全桌面上的程序，也會進 AppContainer 的商店應用（Low）。在這些宿主裡
//! `ShellExecute` 要麼起出一個 SYSTEM 許可權、掛在安全桌面上的 Server，要麼乾脆起不來——
//! 只在普通桌面應用（Medium）里拉，其餘照舊退避重連。
//!
//! 安裝 / 解除安裝程式執行期間也不拉：它剛結束 Server 要替換 exe，這時拉起會佔住檔案（錯誤程式碼 5），
//! 安裝程式持有命名互斥體 [`INSTALLER_MUTEX`]（見 `qingjian.iss`），裝完它自己起新 Server。

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, GetLastError, HANDLE,
};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
    TOKEN_QUERY, TokenIntegrityLevel,
};
use windows::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, OpenMutexW, OpenProcessToken, SYNCHRONIZATION_SYNCHRONIZE,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

use crate::com::log::log;
use crate::com::module_path;

/// 同一程序內兩次嘗試拉起 Server 的最短間隔：連線失敗每個鍵都會走到這裡，別把應用砸出一串 Server。
const LAUNCH_COOLDOWN: Duration = Duration::from_secs(5);

/// 跨程序互斥體：多個應用的 DLL 同時發現 Server 不在時只起一個（第二個起來的搶不到管道會自己退出）。
const LAUNCH_MUTEX: windows::core::PCWSTR = w!("Local\\QingjianServerLaunch");

/// 安裝 / 解除安裝程式執行期間持有的互斥體，名字與 `qingjian.iss` 的 `HoldInstallerMutex` 一致。
const INSTALLER_MUTEX: windows::core::PCWSTR = w!("Global\\QingjianInstaller");

/// 普通桌面應用的完整性級別 RID（UAC 未提升的使用者程序）。低一檔是 AppContainer / 瀏覽器沙箱，
/// 高一檔是管理員提升、SYSTEM 與安全桌面上的程序——那些裡都不拉 Server。
const MEDIUM_INTEGRITY_RID: u32 = 0x2000;

/// 上次嘗試拉起的時間；本程序內所有文本服務例項共用。
static LAST_LAUNCH: Mutex<Option<Instant>> = Mutex::new(None);

/// 連不上 Server 時拉起它。已請求啟動返回 `true`（下一鍵就該連上），沒試 / 失敗返回 `false`。
pub(super) fn launch_server() -> bool {
    if !cooldown_passed() {
        return false;
    }
    if !host_is_plain_desktop_app() {
        log("宿主程序不是普通桌面應用（完整性級別非 Medium），不拉起 Server");
        return false;
    }
    if installer_running() {
        log("安裝程式正在執行，不拉起 Server");
        return false;
    }
    let Some(exe) = server_exe() else {
        log("找不到與 DLL 同目錄的 qingjian-server.exe，不拉起");
        return false;
    };
    let Some(_mutex) = launch_mutex() else {
        // 別的程序正在起：不重複起，退回退避重連
        return false;
    };
    let file = HSTRING::from(exe.as_os_str());
    let workdir = exe
        .parent()
        .map(|dir| HSTRING::from(dir.as_os_str()))
        .unwrap_or_default();
    // 起完就返回：Server 一百多毫秒就監聽管道，阻塞在應用 UI 執行緒上會被 TSF 看門狗切走輸入法。
    let code = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR(workdir.as_ptr()),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW 返回值 > 32 才算成功。
    if (code.0 as isize) > 32 {
        log("已請求啟動 qingjian-server");
        true
    } else {
        log(&format!(
            "啟動 qingjian-server 失敗，返回值 {}",
            code.0 as isize
        ));
        false
    }
}

/// 程序內冷卻：距上次嘗試不到 [`LAUNCH_COOLDOWN`] 就不試。時間在嘗試**之前**記，失敗也冷卻。
fn cooldown_passed() -> bool {
    let Ok(mut last) = LAST_LAUNCH.lock() else {
        return false;
    };
    if last.is_some_and(|at| at.elapsed() < LAUNCH_COOLDOWN) {
        return false;
    }
    *last = Some(Instant::now());
    true
}

/// 宿主程序是不是普通桌面應用（完整性級別為 Medium）。
///
/// 讀當前程序令牌的 `TokenIntegrityLevel`，SID 最後一個子授權值就是級別 RID。
/// 讀不到一律當成「不是」：寧可不拉（退回退避重連，下次登入時由啟動資料夾的快捷方式起），
/// 也不能在安全桌面上起出一個 SYSTEM 許可權的 Server。
fn host_is_plain_desktop_app() -> bool {
    let mut token = HANDLE::default();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
        return false;
    }
    let mut needed = 0u32;
    // 兩段式：第一次只問要多少位元組（總是報緩衝區不夠，屬正常）。
    unsafe {
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut needed);
    }
    let mut buffer = vec![0u8; needed as usize];
    let queried = needed > 0
        && unsafe {
            GetTokenInformation(
                token,
                TokenIntegrityLevel,
                Some(buffer.as_mut_ptr().cast()),
                needed,
                &mut needed,
            )
        }
        .is_ok();
    let _ = unsafe { CloseHandle(token) };
    if !queried {
        return false;
    }
    let label = unsafe {
        buffer
            .as_ptr()
            .cast::<TOKEN_MANDATORY_LABEL>()
            .read_unaligned()
    };
    let sid = label.Label.Sid;
    let count_ptr = unsafe { GetSidSubAuthorityCount(sid) };
    if count_ptr.is_null() {
        return false;
    }
    let count = unsafe { *count_ptr };
    if count == 0 {
        return false;
    }
    let rid_ptr = unsafe { GetSidSubAuthority(sid, u32::from(count) - 1) };
    !rid_ptr.is_null() && unsafe { *rid_ptr } == MEDIUM_INTEGRITY_RID
}

/// 安裝程式的互斥體在不在。它由提升的安裝程式建，普通應用開啟會被拒（`ERROR_ACCESS_DENIED`），拒也說明存在。
fn installer_running() -> bool {
    match unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, INSTALLER_MUTEX) } {
        Ok(handle) => {
            let _ = unsafe { CloseHandle(handle) };
            true
        }
        Err(error) => error.code() == ERROR_ACCESS_DENIED.to_hresult(),
    }
}

/// 跨程序互斥體；已被別的程序持有（對方正在起 Server）返回 `None`。
fn launch_mutex() -> Option<LaunchMutex> {
    let handle = unsafe { CreateMutexW(None, false, LAUNCH_MUTEX) }.ok()?;
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        let _ = unsafe { CloseHandle(handle) };
        return None;
    }
    Some(LaunchMutex(handle))
}

/// 持有互斥體的控制代碼，析構時關閉。
struct LaunchMutex(HANDLE);

impl Drop for LaunchMutex {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn server_exe() -> Option<PathBuf> {
    let module = module_path().ok()?;
    Some(server_exe_path(Path::new(&module.to_string())))
}

/// 與 DLL 同目錄的 `qingjian-server.exe`（安裝器把兩者裝在同一目錄）。
fn server_exe_path(module: &Path) -> PathBuf {
    module.with_file_name("qingjian-server.exe")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::server_exe_path;

    #[test]
    fn server_exe_sits_next_to_the_dll() {
        assert_eq!(
            server_exe_path(Path::new(
                r"D:\Program Files\Qingjian\qingjian_tsf-0.1.0-alpha.15-dev.dll"
            )),
            Path::new(r"D:\Program Files\Qingjian\qingjian-server.exe")
        );
    }
}
