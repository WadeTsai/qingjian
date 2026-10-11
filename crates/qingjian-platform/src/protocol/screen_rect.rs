use serde::{Deserialize, Serialize};

/// 螢幕座標系裡的一個矩形（畫素）。DLL 在編輯會話裡量到組句範圍的螢幕位置後，用
/// [`super::ClientMessage::PositionCandidates`] 發給 Server；Server 據此把候選視窗擺到游標下方。
///
/// 對應 Win32 的 `RECT`，但不依賴 `windows` crate、可 serde，兩端共用（候選視窗自繪搬到了 Server 程序，
/// 才能蓋過微軟商店 / 工作列搜尋這些高 z-band 宿主）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRect {
    /// 左邊界。
    pub left: i32,

    /// 上邊界。
    pub top: i32,

    /// 右邊界。
    pub right: i32,

    /// 下邊界。
    pub bottom: i32,
}
