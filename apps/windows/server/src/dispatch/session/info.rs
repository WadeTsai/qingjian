//! 会话信息。

/// 一个活跃会话在 Server 侧记下的信息，开会话时由 DLL 报来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionInfo {
    /// 宿主应用的 exe 文件名（如 `Code.exe`），查 `[apps]` 用；取不到为 `None`。
    pub(crate) app: Option<String>,

    /// 该会话当前落在私密输入框里（DLL 随 `ClientMessage::Privacy` 报来）；焦点切回来时按它重设 Engine。
    pub(crate) private: bool,

    /// DLL 报来的协议版本。比 Server 低的（应用还没重启、加载着旧 DLL）要按老协议发帧，
    /// 见 composed 里的 downgrade_for_old_dll。
    pub(crate) protocol: u32,

    /// 该会话用的是繁体那个语言 profile（「中文(台灣)」下的青簡，DLL 随 `ClientMessage::Profile` 报来）：
    /// 不看 `[general] traditional` 就出繁体。开会话时按简体算，DLL 只在是繁体时才报。
    pub(crate) traditional: bool,
}
