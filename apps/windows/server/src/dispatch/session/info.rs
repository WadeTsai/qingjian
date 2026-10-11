//! 會話資訊。

/// 一個活躍會話在 Server 側記下的資訊，開會話時由 DLL 報來。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionInfo {
    /// 宿主應用的 exe 檔名（如 `Code.exe`），查 `[apps]` 用；取不到為 `None`。
    pub(crate) app: Option<String>,

    /// 該會話當前落在私密輸入框裡（DLL 隨 `ClientMessage::Privacy` 報來）；焦點切回來時按它重設 Engine。
    pub(crate) private: bool,

    /// DLL 報來的協議版本。比 Server 低的（應用還沒重啟、載入著舊 DLL）要按老協議發幀，
    /// 見 composed 裡的 downgrade_for_old_dll。
    pub(crate) protocol: u32,

    /// 該會話用的是繁體那個語言 profile（「中文(台灣)」下的青簡，DLL 隨 `ClientMessage::Profile` 報來）：
    /// 不看 `[general] traditional` 就出繁體。開會話時按簡體算，DLL 只在是繁體時才報。
    pub(crate) traditional: bool,
}
