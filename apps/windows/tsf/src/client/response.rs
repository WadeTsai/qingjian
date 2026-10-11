use qingjian_platform::protocol::{Frame, IndicatorState, InputSettings, KeyOutcome};

/// Server 對一次「同步中英模式」輪詢的答覆。
pub struct ModeSyncReply {
    /// `Some(true)` 切英文、`Some(false)` 切中文；`None` 沒有待處理的切換。
    pub english: Option<bool>,

    /// 當前的按鍵行為設定（切換鍵、內建英文模式）。每一拍都帶，配置改了靠它生效——
    /// DLL 不讀配置檔案，`%APPDATA%\Qingjian` 對 AppContainer 裡的商店應用本來也讀不到。
    pub input: InputSettings,

    /// 右鍵選單打勾用的開關狀態，同樣每一拍都帶。
    pub indicator: IndicatorState,
}

/// Server 對一次按鍵的處理結果。
pub struct KeyResponse {
    /// 吃掉還是放行給應用。
    pub outcome: KeyOutcome,

    /// 本次要立即上屏到文件的文本。
    pub commit: Option<String>,

    /// 處理後的組句狀態（preedit + 候選）；空幀表示收起候選視窗。
    pub frame: Frame,
}
