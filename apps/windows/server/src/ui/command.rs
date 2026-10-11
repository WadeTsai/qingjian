//! 交給 UI 執行緒執行的命令。

use qingjian_platform::protocol::{Frame, ScreenRect};
use qingjian_render::Mode;

use crate::dispatch::{RenderSettings, StatusView};

/// 交給 UI 執行緒執行的命令。`Frame` 較大，裝箱免得列舉過胖。
pub(super) enum UiCommand {
    /// 把候選視窗擺到組句矩形下方並按幀重繪。
    Show(Box<(Frame, Mode, ScreenRect)>),

    /// 收起候選視窗。
    Hide,

    /// 顯示 / 更新懸浮狀態條。
    StatusShow(Box<StatusView>),

    /// 收起懸浮狀態條。
    StatusHide,

    /// 換畫法（渲染器 / 字型）。
    Configure(RenderSettings),
}
