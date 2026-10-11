//! 渲染器：一幀 + 排布 + 主題 → 點陣圖。排版與 macOS 殼的 `CandidateView` 一致：頂部拼音行，豎排一行一個候選、橫排排成一行。
//!
//! 先按幀建一棵場景樹（[`Scene`]），Taffy 算佈局，再按樹序畫。內部全用畫素：主題裡的點數進來先乘縮放倍數。
//! 文位元組點的盒子頂邊就是行框頂邊，字形在行高裡垂直居中。

mod animate;
mod build;
mod hit;
mod partial;
mod rendered;
mod retained;
mod status;

use std::path::PathBuf;
use std::time::Instant;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::error::RenderError;
use crate::fonts::FontLibrary;
use crate::frame::Frame;
use crate::layout::Layout;
use crate::scene::Scene;
use crate::text::{TextPainter, TextStyle};
use crate::theme::{FontSpec, Theme};

use build::Builder;
use retained::Retained;

pub use hit::{HitRegion, HitTarget};
pub use rendered::Rendered;
pub use status::{RenderedStatus, StatusCell};

/// 光學字號（點）：20 pt 以下 CoreText 給系統字型用的就是這一檔。
const OPTICAL_SIZE: f32 = 17.0;

/// 候選詞的文字樣式名：狀態條之外只有核對字型回退時直接用到。
const CANDIDATE_FONT: &str = "candidate";

pub struct Renderer {
    /// 文字測繪。
    text: TextPainter,

    /// 上一幀候選窗（過渡配對與動畫幀重畫用）；視窗隱藏後清掉。
    last: Option<Retained>,

    /// 系統開了「減少動態效果」：不播過渡，迴圈動畫停在開頭。
    reduce_motion: bool,

    /// 最近一幀的主題關了動畫（設定裡的開關），效果同上。
    animations_off: bool,

    /// 迴圈動畫的時鐘起點：窗口出現後的第一幀；打字過程中不重置，視窗收起（`forget`）歸零。
    clock: Option<Instant>,
}

impl Renderer {
    pub fn new(library: FontLibrary) -> Self {
        let mut text = TextPainter::new(library);
        text.set_optical_size(Some(OPTICAL_SIZE));
        Self {
            text,
            last: None,
            reduce_motion: false,
            animations_off: false,
            clock: None,
        }
    }

    /// 載入主題要的字型：隨主題帶的檔案，加上樣式裡寫到、字型庫裡還沒有的系統字族（`family_files` 由殼按平臺的字型登記查檔案）。
    /// 換主題時調；同一個檔案只加載一次。字型沒裝的樣式按回退鏈往後找，最後用介面字型。
    pub fn load_theme_fonts(
        &mut self,
        theme: &Theme,
        mut family_files: impl FnMut(&str) -> Vec<PathBuf>,
    ) {
        self.text.load_fonts(theme.font_files());
        for family in theme.font_families() {
            if !self.text.has_family(&family) {
                self.text.load_fonts(&family_files(&family));
            }
        }
    }

    /// 畫一幀。`scale` 是點 → 畫素的倍數（Retina 為 2）；點陣圖按畫出範圍開（投影、伸出的裝飾），根節點的盒子是內容區。
    /// 主題裡帶過渡的節點與上一幀配對，位置變了就從舊位置出發，[`Rendered::next_frame`] 告訴殼多久後要下一幀。
    pub fn render(
        &mut self,
        frame: &Frame,
        layout: Layout,
        theme: &Theme,
        scale: f32,
    ) -> Result<Rendered, RenderError> {
        self.render_at(frame, layout, theme, scale, Instant::now())
    }

    /// 同 [`Self::render`]，時間由呼叫方給（測試用固定時刻）。
    pub fn render_at(
        &mut self,
        frame: &Frame,
        layout: Layout,
        theme: &Theme,
        scale: f32,
        now: Instant,
    ) -> Result<Rendered, RenderError> {
        self.text.use_families(theme.families());
        self.animations_off = !theme.animations();
        let mut scene = Scene::new();
        let mut builder = Builder {
            scene: &mut scene,
            theme,
            scale,
            text: &mut self.text,
            cell_nodes: Vec::new(),
            candidate_nodes: Vec::new(),
            sense_nodes: Vec::new(),
        };
        let root = builder.window(frame, layout)?;
        let (candidate_nodes, sense_nodes) = (builder.candidate_nodes, builder.sense_nodes);
        let (content_width, content_height) = scene.layout(root, &mut self.text)?;
        let hits = hit_regions(&scene, root, &candidate_nodes, &sense_nodes)?;
        let keyed = scene.keyed(root)?;
        let transitions = self.transitions(&keyed, layout, scale, now);
        let animated = scene.animated(root)?;
        // 畫出範圍按起止兩頭的並算，迴圈動畫按一輪裡能到的最遠處算：動畫中途點陣圖不變大小
        let mut extent = scene.extent(root)?;
        for transition in &transitions {
            let from = transition.from;
            let reach = scene.reach(transition.node).unwrap_or(0.0);
            extent.include((from.x, from.y, from.width, from.height), reach);
        }
        for (node, keyframes) in &animated {
            let parent = scene.parent_origin(*node, root)?;
            let own = scene.subtree_extent(*node, parent.0, parent.1)?;
            let (cx, cy) = scene.center(*node, parent.0, parent.1)?;
            let (dx, dy, grow) = keyframes.reach();
            // 旋轉取外接圓，縮放按最大倍數，都繞節點中心
            let (half_w, half_h) = ((own.right - own.left) / 2.0, (own.bottom - own.top) / 2.0);
            let (half_w, half_h) = if keyframes.rotates() {
                let r = (half_w * half_w + half_h * half_h).sqrt();
                (r, r)
            } else {
                (half_w, half_h)
            };
            let (ox, oy) = (
                (own.left + own.right) / 2.0 - cx,
                (own.top + own.bottom) / 2.0 - cy,
            );
            let reach_x = (ox.abs() + half_w) * grow.max(1.0) + dx;
            let reach_y = (oy.abs() + half_h) * grow.max(1.0) + dy;
            extent.include(
                (cx - reach_x, cy - reach_y, reach_x * 2.0, reach_y * 2.0),
                0.0,
            );
        }
        let (x, y) = ((-extent.left).ceil(), (-extent.top).ceil());
        let mut retained = Retained {
            scene,
            root,
            layout,
            scale,
            size: (
                (extent.right + x).ceil() as u32,
                (extent.bottom + y).ceil() as u32,
            ),
            origin: (x, y),
            content: (content_width.ceil(), content_height.ceil()),
            placements: keyed
                .iter()
                .map(|keyed| (keyed.key.clone(), keyed.placement))
                .collect(),
            transitions,
            animated,
            partial: None,
            hits,
        };
        if self.clock.is_none() {
            self.clock = Some(now);
        }
        let rendered = self.paint_retained(&mut retained, now);
        self.last = Some(retained);
        rendered
    }

    /// 按場景畫出來的範圍開點陣圖（投影、伸出視窗的裝飾都在裡面），根節點的盒子是內容區。
    fn rasterize(
        &mut self,
        scene: &Scene,
        root: taffy::NodeId,
        (content_width, content_height): (f32, f32),
        scale: f32,
    ) -> Result<Rendered, RenderError> {
        let extent = scene.extent(root)?;
        let (x, y) = ((-extent.left).ceil(), (-extent.top).ceil());
        let width = (extent.right + x).ceil();
        let height = (extent.bottom + y).ceil();
        let mut canvas = Canvas::new(width as u32, height as u32)?;
        scene.paint(root, &mut canvas, &mut self.text, x, y)?;
        Ok(Rendered {
            pixmap: canvas.into_pixmap(),
            content_x: x as u32,
            content_y: y as u32,
            content_width: content_width as u32,
            content_height: content_height as u32,
            scale,
            next_frame: None,
            hits: Vec::new(),
        })
    }

    /// 量一段文字在 `size` 點字號下的寬度（點），與原生排版的數值對照用。
    pub fn measure_points(&mut self, text: &str, size: f32) -> f32 {
        let style = TextStyle::new(FontSpec::new(size, size), size, Color::rgb(0, 0, 0), 1.0);
        self.text.measure(text, &style).width
    }

    /// 每個字形用到的字族名（按候選詞的字號），驗證回退鏈用。
    pub fn trace_families(&mut self, text: &str, theme: &Theme) -> Vec<String> {
        self.text.use_families(theme.families());
        let font = theme.font(CANDIDATE_FONT);
        let style = TextStyle::new(font, font.size, Color::rgb(0, 0, 0), theme.text_gamma());
        self.text.trace_families(text, &style)
    }
}

/// 各候選與各條譯詞的點選區域（內容區畫素）：同一個候選 / 同一條譯詞的節點取外接矩形，譯詞排在前面先命中。
fn hit_regions(
    scene: &Scene,
    root: taffy::NodeId,
    candidates: &[(usize, taffy::NodeId)],
    senses: &[(usize, usize, taffy::NodeId)],
) -> Result<Vec<HitRegion>, RenderError> {
    let mut regions: Vec<HitRegion> = Vec::new();
    let mut add = |target: HitTarget, node: taffy::NodeId| -> Result<(), RenderError> {
        let rect = scene.rect_in(node, root)?;
        match regions.iter_mut().find(|region| region.target == target) {
            Some(region) => *region = region.union(rect),
            None => regions.push(HitRegion {
                x: rect.0,
                y: rect.1,
                width: rect.2,
                height: rect.3,
                target,
            }),
        }
        Ok(())
    };
    for &(candidate, sense, node) in senses {
        add(HitTarget::Translation { candidate, sense }, node)?;
    }
    for &(candidate, node) in candidates {
        add(HitTarget::Candidate(candidate), node)?;
    }
    Ok(regions)
}
