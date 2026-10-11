//! 設定程式只開一個：第二次啟動把已開的視窗帶到前臺後退出。

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::w;

/// 已有例項在跑就喚起它並返回 `false`；這是第一個例項返回 `true`。
///
/// 互斥體控制代碼故意不關，程序退出時由系統回收。
pub(crate) fn acquire() -> bool {
    let created = unsafe { CreateMutexW(None, false, w!("Local\\QingjianSettings")) };
    if created.is_err() || unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
        return true;
    }
    // 第一個例項還沒建出視窗（連點兩下）就找不到，直接退出即可。
    if let Ok(hwnd) = unsafe { FindWindowW(None, w!("青簡設定")) } {
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }
    false
}
