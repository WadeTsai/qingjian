//! 候選視窗的輸出端。

use qingjian_platform::protocol::{Frame, ScreenRect};
use qingjian_platform::{CandidateRenderer, FontSize};
use qingjian_render::Mode;

/// 候選視窗 / 狀態條的畫法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSettings {
    /// 由誰畫（`[general] renderer`）。
    pub renderer: CandidateRenderer,

    /// 字族名（`[general] font`），空為系統字型。
    pub font: String,

    /// 主題 id（`[general] theme`，已按舊寫法相容解析）。
    pub theme: String,

    /// 候選字、譯文字號（`[general] candidate_font_size` / `annotation_font_size`），蓋過主題。
    pub font_sizes: (FontSize, FontSize),

    /// 過渡動畫（`[general] animations`）。
    pub animations: bool,

    /// 使用者主題目錄的戳（`ThemeLibrary::stamp`）：主題檔案改了它就變，UI 執行緒據此重讀主題。
    pub themes_stamp: u64,
}

/// Router 只產出幀，畫交給它；Windows 上由 UI 執行緒實現。
pub trait CandidateSink: Send {
    /// 把候選視窗擺到 `rect`（組句範圍的螢幕矩形）下方並按 `frame` 重繪；`mode` 給主題顯示中 / 英等狀態。
    fn show(&self, frame: Frame, mode: Mode, rect: ScreenRect);

    fn hide(&self);

    /// 換畫法：裝上時與配置熱載入後調，只在設定變了時調。
    fn configure(&self, settings: RenderSettings);
}

/// 不畫候選視窗的空實現。
pub struct NoopSink;

impl CandidateSink for NoopSink {
    fn show(&self, _frame: Frame, _mode: Mode, _rect: ScreenRect) {}

    fn hide(&self) {}

    fn configure(&self, _settings: RenderSettings) {}
}
