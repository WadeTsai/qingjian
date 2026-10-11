use serde::{Deserialize, Serialize};

/// Server 對一次按鍵的處置：DLL 據此決定 TSF 裡 `OnKeyDown` 返回吃掉還是放行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyOutcome {
    /// 輸入法消費了這次按鍵（進了組句緩衝或觸發了上屏 / 翻頁等）；DLL 吃掉，應用收不到。
    Consumed,

    /// 輸入法不處理（如沒在組句時的普通字元、快捷鍵）；DLL 放行給應用。
    Passthrough,
}
