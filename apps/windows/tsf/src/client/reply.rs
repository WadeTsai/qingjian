use super::KeyResponse;

/// 一次 [`super::EngineClient::key`] 的應答。
pub enum KeyReply {
    /// 常規處理結果。
    Result(KeyResponse),

    /// 收到「翻譯選中文字」快捷鍵：Server 請 DLL 讀當前選區。
    NeedSelection {
        /// 請求標識，隨 [`qingjian_platform::protocol::ClientMessage::Selection`] 帶回。
        request: u64,
    },
}
