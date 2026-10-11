//! 青簡渲染器在 Windows 殼裡的落地：字型庫 + 渲染器一份，候選視窗與狀態條共用（字形快取共享）。
//! 配置 `[general] renderer = "system"` 時沒有它，兩個視窗走原來的 GDI 畫法（過渡期退路）。

use std::cell::RefCell;
use std::rc::Rc;

use qingjian_platform::{CandidateRenderer, LayoutMode};
use qingjian_render::{
    FontLibrary, Frame, Layout, Mode, Rendered, RenderedStatus, Renderer, StatusCell, TextSizes,
    Theme, ThemeLibrary, UiFont, system_fonts,
};

use crate::dispatch::RenderSettings;

/// UI 執行緒上共享的渲染器；`None` = 系統繪製。
pub(super) type SharedPainter = Rc<RefCell<Option<Painter>>>;

pub(super) struct Painter {
    /// 渲染器（字型庫隨它）。
    renderer: Renderer,

    /// 建它時用的字族名（空為系統字型），設定沒變就不重建。
    font: String,

    /// 主題（淺色那一份，畫的時候按外觀取深淺）。
    theme: Theme,
}

impl Painter {
    /// `font` 是使用者選的字族名，空為系統字型；沒裝就回到系統字型。字型庫載入失敗返回 `None`，呼叫方退回 GDI。
    fn new(font: &str) -> Option<Self> {
        let started = std::time::Instant::now();
        let library = if font.is_empty() {
            FontLibrary::system("zh-CN")
        } else {
            let ui_font = UiFont {
                family: font.to_owned(),
                files: system_fonts::family_files(font),
            };
            FontLibrary::with_ui_font("zh-CN", &ui_font)
        };
        let library = match library {
            Ok(library) => library,
            Err(error) => {
                tracing::warn!(%error, "渲染器字型庫載入失敗，候選視窗與狀態條退回 GDI 繪製");
                return None;
            }
        };
        tracing::info!(
            elapsed = ?started.elapsed(),
            font = library.ui_family(),
            "候選視窗與狀態條使用青簡渲染器"
        );
        Some(Self {
            renderer: Renderer::new(library),
            font: font.to_owned(),
            theme: Theme::light(),
        })
    }

    /// 按設定建 / 換 / 撤渲染器。
    pub(super) fn configure(shared: &SharedPainter, settings: &RenderSettings) {
        let mut painter = shared.borrow_mut();
        match settings.renderer {
            CandidateRenderer::Qingjian => {
                if painter.as_ref().map(|p| p.font.as_str()) != Some(settings.font.as_str()) {
                    *painter = Self::new(&settings.font);
                }
                // 設定變了才會走到這裡（含主題檔案的戳），每次都從主題目錄重讀
                if let Some(painter) = painter.as_mut() {
                    let themes =
                        ThemeLibrary::load(qingjian_platform::dirs::themes_dir().as_deref());
                    let (candidate, annotation) = settings.font_sizes;
                    painter.theme = themes
                        .resolve(&settings.theme, false)
                        .with_text_sizes(TextSizes {
                            candidate: candidate.get(),
                            annotation: annotation.get(),
                        })
                        .with_animations(settings.animations);
                    painter
                        .renderer
                        .load_theme_fonts(&painter.theme, system_fonts::family_files);
                }
            }
            CandidateRenderer::System => {
                if painter.is_some() {
                    tracing::info!("候選視窗與狀態條切回 GDI 繪製");
                    *painter = None;
                }
            }
        }
    }

    /// 畫一幀候選視窗；`dpi` 96 為 100%。失敗記日誌返回 `None`，呼叫方退回 GDI。
    pub(super) fn render_frame(
        &mut self,
        frame: &Frame,
        layout: LayoutMode,
        dark: bool,
        dpi: u32,
    ) -> Option<Rendered> {
        let layout = match layout {
            LayoutMode::Vertical => Layout::Vertical,
            LayoutMode::Horizontal => Layout::Horizontal,
        };
        let started = std::time::Instant::now();
        self.renderer.set_reduce_motion(reduce_motion());
        let rendered = self
            .renderer
            .render(frame, layout, &self.theme.with_dark(dark), scale(dpi))
            .inspect_err(|error| tracing::warn!(%error, "候選窗渲染失敗"))
            .ok()?;
        tracing::debug!(
            elapsed = ?started.elapsed(),
            width = rendered.content_width,
            height = rendered.content_height,
            "候選窗點陣圖已畫"
        );
        Some(rendered)
    }

    /// 動畫的下一幀；沒有在播的返回 `None`。
    pub(super) fn tick(&mut self) -> Option<Rendered> {
        self.renderer
            .tick()
            .inspect_err(|error| tracing::warn!(%error, "候選窗動畫幀渲染失敗"))
            .ok()
            .flatten()
    }

    /// 候選窗收起：忘掉上一幀，下次顯示不從舊位置過渡。
    pub(super) fn forget(&mut self) {
        self.renderer.forget();
    }

    /// 畫狀態條。
    pub(super) fn render_status(
        &mut self,
        cells: &[StatusCell],
        mode: &Mode,
        dark: bool,
        dpi: u32,
    ) -> Option<RenderedStatus> {
        self.renderer
            .render_status(cells, mode, &self.theme.with_dark(dark), scale(dpi))
            .inspect_err(|error| tracing::warn!(%error, "狀態條渲染失敗"))
            .ok()
    }
}

/// 系統關了「動畫效果」（設定 → 輔助功能 → 視覺效果）：不播過渡。讀不到當開著。
fn reduce_motion() -> bool {
    let mut enabled = windows::core::BOOL(1);
    // SAFETY: SPI_GETCLIENTAREAANIMATION 往 pvParam 寫一個 BOOL。
    let ok = unsafe {
        windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            windows::Win32::UI::WindowsAndMessaging::SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut enabled).cast()),
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    ok.is_ok() && !enabled.as_bool()
}

/// 點 → 畫素的倍數。
fn scale(dpi: u32) -> f32 {
    dpi.max(96) as f32 / 96.0
}
