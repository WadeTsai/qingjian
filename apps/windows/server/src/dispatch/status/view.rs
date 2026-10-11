use qingjian_platform::Appearance;
use qingjian_render::Mode;

/// 狀態條一次要顯示的內容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    /// 英文模式（`false` 中文）。
    pub english: bool,

    /// 是否啟用大千注音。
    pub zhuyin: bool,

    /// 開著雙拼時的方案名，中文格里跟在「中」後面。
    pub scheme: Option<String>,

    /// 當前模式的全形標點開著（中英各記一份配置）；關著時格子顯示 `,.` 畫成灰的。
    pub full_width: bool,

    /// 外觀：跟隨系統 / 淺色 / 深色。
    pub appearance: Appearance,

    /// 配置裡記住的內容左上角物理畫素；`None` 首次按螢幕右下角擺。
    pub anchor: Option<(i32, i32)>,

    /// 輸入狀態，給節點樹畫法的主題顯示（與候選視窗同一份）。
    pub mode: Mode,
}
