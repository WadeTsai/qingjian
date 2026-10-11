//! 候選視窗的點陣圖繪製：一幀交給 `qingjian-render` 畫成點陣圖，`drawRect:` 裡貼上去。
//!
//! 與自繪 NSView 的舊路徑並存：配置 `[general] renderer = "system"` 走舊路（過渡期退路）。
//! 陰影由主題定義、渲染器畫進點陣圖（點陣圖四周留邊），面板關掉系統陰影，與 Windows 一致。

mod convert;
mod font_files;

pub(crate) use font_files::{available_families, family_files};

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{NSBitmapImageRep, NSCalibratedRGBColorSpace, NSCompositingOperation, NSImage};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use qingjian_platform::LayoutMode;
use qingjian_render::{FontLibrary, HitRegion, HitTarget, Layout, Renderer, Theme, UiFont};

use super::bounds::ViewBounds;
use super::frame::Frame;

pub struct BitmapPainter {
    /// 渲染器（字型庫隨它）。
    renderer: Renderer,

    /// 最近一幀的點陣圖，`None` 表示還沒畫過。
    image: Option<Retained<NSImage>>,

    /// 最近一幀（外觀變了要重畫）。
    frame: qingjian_render::Frame,

    /// 最近一幀的排布。
    layout: Layout,

    /// 最近一幀按深色畫的。
    dark: bool,

    /// 最近一幀的倍數。
    scale: f32,

    /// 最近一幀的尺寸（點）與其中的內容區。
    bounds: ViewBounds,

    /// 主題（淺色那一份，畫的時候按外觀取深淺）。
    theme: Theme,

    /// 最近一幀還有動畫在播：隔多久要調 [`Self::tick`]。
    next_frame: Option<std::time::Duration>,

    /// 最近一幀裡候選與譯詞的點選區域（內容區畫素）。
    hits: Vec<HitRegion>,

    /// 內容區左上角在視圖裡的位置（點，左上為原點）：點陣圖四周留了投影的邊。
    content_origin: NSPoint,
}

impl BitmapPainter {
    /// `font` 是使用者選的字族名，空為系統字型；沒裝就回到系統字型。字型庫載入失敗返回 `None`，呼叫方退回舊路徑。
    pub fn new(font: &str) -> Option<Self> {
        let started = std::time::Instant::now();
        let font = font.trim();
        let library = if font.is_empty() {
            FontLibrary::system("zh-CN")
        } else {
            let ui_font = UiFont {
                family: font.to_owned(),
                files: font_files::family_files(font),
            };
            FontLibrary::with_ui_font("zh-CN", &ui_font)
        };
        let library = match library {
            Ok(library) => library,
            Err(error) => {
                tracing::warn!(%error, "渲染器字体库加载失败，候选窗退回 AppKit 绘制");
                return None;
            }
        };
        tracing::info!(
            elapsed = ?started.elapsed(),
            font = library.ui_family(),
            "候选窗使用位图渲染器"
        );
        Some(Self {
            renderer: Renderer::new(library),
            image: None,
            frame: qingjian_render::Frame::default(),
            layout: Layout::Vertical,
            dark: false,
            scale: 2.0,
            bounds: ViewBounds::filled(NSSize::ZERO),
            theme: Theme::light(),
            next_frame: None,
            hits: Vec::new(),
            content_origin: NSPoint::ZERO,
        })
    }

    /// 換主題（淺色那一份）：載入它要的字型，用最近一幀當場重畫。
    pub fn set_theme(&mut self, theme: Theme) {
        self.renderer
            .load_theme_fonts(&theme, font_files::family_files);
        self.theme = theme;
        self.repaint();
    }

    /// 記下新一幀並畫好，返回檢視該有的尺寸與內容區（點）。
    pub fn set_frame(
        &mut self,
        frame: &Frame,
        layout: LayoutMode,
        dark: bool,
        scale: f32,
    ) -> ViewBounds {
        self.frame = convert::frame(frame);
        self.renderer.set_reduce_motion(reduce_motion());
        self.layout = match layout {
            LayoutMode::Vertical => Layout::Vertical,
            LayoutMode::Horizontal => Layout::Horizontal,
        };
        self.dark = dark;
        self.scale = scale;
        self.repaint();
        self.bounds
    }

    /// 外觀或倍數變了就重畫一遍再貼。
    pub fn draw(&mut self, dark: bool, scale: f32) {
        if dark != self.dark || scale != self.scale {
            self.dark = dark;
            self.scale = scale;
            self.repaint();
        }
        let Some(image) = &self.image else {
            return;
        };
        let rect = NSRect::new(NSPoint::ZERO, self.bounds.size);
        // SAFETY: hints 傳 None，其餘引數都是普通值；在 drawRect: 內呼叫，有當前圖形上下文。
        unsafe {
            image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                rect,
                NSRect::ZERO,
                NSCompositingOperation::Copy,
                1.0,
                true,
                None,
            );
        }
    }

    /// 視圖裡一點（點，左上為原點）點中了哪個候選或哪條譯詞。
    pub fn hit(&self, point: NSPoint) -> Option<HitTarget> {
        let scale = f64::from(self.scale);
        let x = ((point.x - self.content_origin.x) * scale) as f32;
        let y = ((point.y - self.content_origin.y) * scale) as f32;
        self.hits
            .iter()
            .find(|region| region.contains(x, y))
            .map(|region| region.target)
    }

    /// 有動畫在播時隔多久要下一幀。
    pub fn next_frame(&self) -> Option<std::time::Duration> {
        self.next_frame
    }

    /// 動畫的下一幀：換上新點陣圖（尺寸不變）；返回隔多久再要，`None` 為播完。
    pub fn tick(&mut self) -> Option<std::time::Duration> {
        match self.renderer.tick() {
            Ok(Some(rendered)) => {
                self.next_frame = rendered.next_frame;
                self.image = to_image(&rendered.pixmap, self.bounds.size);
            }
            Ok(None) => self.next_frame = None,
            Err(error) => {
                tracing::warn!(%error, "候选窗动画帧渲染失败");
                self.next_frame = None;
            }
        }
        self.next_frame
    }

    /// 視窗收起：停動畫、忘掉上一幀，下次顯示不從舊位置過渡。
    pub fn forget(&mut self) {
        self.renderer.forget();
        self.next_frame = None;
    }

    fn repaint(&mut self) {
        let theme = self.theme.with_dark(self.dark);
        let started = std::time::Instant::now();
        let rendered = match self
            .renderer
            .render(&self.frame, self.layout, &theme, self.scale)
        {
            Ok(rendered) => rendered,
            Err(error) => {
                tracing::warn!(%error, "候选窗渲染失败");
                self.image = None;
                self.hits.clear();
                return;
            }
        };
        let (width, height) = rendered.content_size_points();
        self.bounds = bounds(&rendered);
        self.next_frame = rendered.next_frame;
        self.content_origin = NSPoint::new(
            f64::from(rendered.content_x) / f64::from(rendered.scale),
            f64::from(rendered.content_y) / f64::from(rendered.scale),
        );
        self.hits = rendered.hits;
        self.image = to_image(&rendered.pixmap, self.bounds.size);
        tracing::debug!(elapsed = ?started.elapsed(), width, height, "候选窗位图已画");
    }
}

/// 系統輔助功能裡開了「減少動態效果」。
fn reduce_motion() -> bool {
    objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

/// 點陣圖與內容區換成點；內容區原點換成左下角起算。
fn bounds(rendered: &qingjian_render::Rendered) -> ViewBounds {
    let points = |pixels: u32| f64::from(pixels) / f64::from(rendered.scale);
    let (width, height) = (rendered.pixmap.width(), rendered.pixmap.height());
    let below = height.saturating_sub(rendered.content_y + rendered.content_height);
    ViewBounds {
        size: NSSize::new(points(width), points(height)),
        content: NSRect::new(
            NSPoint::new(points(rendered.content_x), points(below)),
            NSSize::new(
                points(rendered.content_width),
                points(rendered.content_height),
            ),
        ),
    }
}

/// 預乘 RGBA 點陣圖 → NSImage（尺寸按點，點陣圖按畫素，Retina 自然對上）。
fn to_image(pixmap: &qingjian_render::Pixmap, size: NSSize) -> Option<Retained<NSImage>> {
    let (width, height) = (pixmap.width(), pixmap.height());
    // SAFETY: planes 傳空讓 AppKit 自己分配；引數描述的是 8 位 × 4 通道、預乘 alpha 在後的連續 RGBA，
    // 與 tiny-skia 的記憶體佈局一致；隨後按 bytesPerRow 逐行拷進去，不越界。
    let rep = unsafe {
        let rep = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            width as isize,
            height as isize,
            8,
            4,
            true,
            false,
            NSCalibratedRGBColorSpace,
            (width * 4) as isize,
            32,
        )?;
        let stride = rep.bytesPerRow() as usize;
        let data = rep.bitmapData();
        let row_bytes = width as usize * 4;
        for (row, source) in pixmap.data().chunks_exact(row_bytes).enumerate() {
            std::ptr::copy_nonoverlapping(source.as_ptr(), data.add(row * stride), row_bytes);
        }
        rep
    };
    let image = NSImage::initWithSize(NSImage::alloc(), size);
    image.addRepresentation(&rep);
    Some(image)
}
