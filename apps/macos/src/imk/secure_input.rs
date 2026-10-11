//! Secure Input（密碼框等）檢測。開著的時候游標附近的文本絕不發往雲端。

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}

/// 系統當前是否處於安全輸入狀態。
pub fn enabled() -> bool {
    // SAFETY: 無引數的純查詢函式，Carbon 框架在 macOS 上始終可用。
    unsafe { IsSecureEventInputEnabled() }
}
