//! 文字測繪：cosmic-text 整形 + swash 柵格，單行、畫素座標；普通字形走覆蓋率遮罩，彩色 emoji 走 RGBA 點陣圖。

mod size;
mod style;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cosmic_text::fontdb::{Family, ID};
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Style, SwashCache, SwashContent};

use crate::canvas::Canvas;
use crate::fonts::{FontLibrary, Trak, UI_FAMILY};
use crate::theme::FontFamilies;
use crate::theme::file::FamilyList;

pub(crate) use size::TextSize;
pub(crate) use style::TextStyle;

pub(crate) struct TextPainter {
    /// 字型庫與回退鏈。
    font_system: FontSystem,

    /// 字形點陣圖快取（按字型、字號、亞畫素位移）。
    cache: SwashCache,

    /// 複用的單行緩衝。
    buffer: Buffer,

    /// 每張字型的字距表（沒有的記 `None`），按字形所用字型各查各的，與 CoreText 一致。
    tracking: HashMap<ID, Option<Trak>>,

    /// 覆蓋率 gamma 查詢表，按 gamma 值快取。
    gamma_tables: HashMap<u32, Box<[u8; 256]>>,

    /// 當前主題的字族表；換了主題或載入了新字型就重新挑。
    families: Option<Arc<FontFamilies>>,

    /// 字族表每條回退鏈挑中的字族名（字型庫裡的寫法），`None` 用介面字型。
    resolved: Vec<Option<String>>,

    /// 主題要求載入過的字型檔案（載入失敗的也記，不反覆試）。
    loaded: HashSet<PathBuf>,
}

impl TextPainter {
    pub(crate) fn new(library: FontLibrary) -> Self {
        let mut font_system = library.into_font_system();
        let buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 19.0));
        Self {
            font_system,
            cache: SwashCache::new(),
            buffer,
            tracking: HashMap::new(),
            gamma_tables: HashMap::new(),
            families: None,
            resolved: Vec::new(),
            loaded: HashSet::new(),
        }
    }

    /// 字型庫裡有沒有這個字族（不分大小寫）。
    pub(crate) fn has_family(&self, name: &str) -> bool {
        self.family_name(name).is_some()
    }

    /// 字型庫裡這個字族的寫法。
    fn family_name(&self, name: &str) -> Option<String> {
        self.font_system.db().faces().find_map(|face| {
            face.families
                .iter()
                .find(|(family, _)| family.eq_ignore_ascii_case(name))
                .map(|(family, _)| family.clone())
        })
    }

    /// 載入字型檔案；每個檔案只試一次。載入了新檔案就重新挑字族。
    pub(crate) fn load_fonts(&mut self, paths: &[PathBuf]) {
        let mut added = false;
        for path in paths {
            if !self.loaded.insert(path.clone()) {
                continue;
            }
            added |= load_font(&mut self.font_system, path);
        }
        if added {
            self.families = None;
        }
    }

    /// 換成這個主題的字族表：每條回退鏈挑第一個字型庫裡有的字族，同一張表不重挑。
    pub(crate) fn use_families(&mut self, families: &Arc<FontFamilies>) {
        if self
            .families
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, families))
        {
            return;
        }
        self.resolved = families
            .chains()
            .iter()
            .map(|chain| {
                for name in &chain.0 {
                    if FamilyList::is_system(name) {
                        return None;
                    }
                    if let Some(found) = self.family_name(name) {
                        return Some(found);
                    }
                }
                tracing::debug!(chain = ?chain.0, "回退链里的字体都没装，用界面字体");
                None
            })
            .collect();
        self.families = Some(Arc::clone(families));
    }

    /// 光學字號（點）：SF 這類帶 `opsz` 軸的字型在小字號用文本視覺尺寸，CoreText 對系統字型自動做，這裡要顯式給。
    /// 現在是整個畫筆一個值（cosmic-text 的字型例項快取沒按它分鍵），候選窗幾種字號都在 20 pt 以下，落到同一檔。
    pub(crate) fn set_optical_size(&mut self, points: Option<f32>) {
        self.font_system.set_optical_size(points);
    }

    /// 一段文字的寬高（畫素）。高度就是行高。
    pub(crate) fn measure(&mut self, text: &str, style: &TextStyle) -> TextSize {
        self.shape(text, style);
        let mut width = 0.0_f32;
        let mut baseline = None;
        for run in self.buffer.layout_runs() {
            baseline.get_or_insert(run.line_y.round());
            let tracked: f32 = run
                .glyphs
                .iter()
                .map(|glyph| {
                    tracking_px(&self.font_system, &mut self.tracking, glyph.font_id, style)
                })
                .sum();
            width = width.max(run.line_w + tracked);
        }
        TextSize {
            width,
            height: style.line_height,
            baseline: baseline.unwrap_or(0.0),
        }
    }

    /// 把文字畫到 `(x, y)`，`y` 是行框頂邊。返回寬度。
    pub(crate) fn draw(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        style: &TextStyle,
        x: f32,
        y: f32,
    ) -> f32 {
        self.shape(text, style);
        if let Some((stroke_width, stroke_color)) = style.stroke {
            self.draw_stroke(canvas, style, x, y, stroke_width, stroke_color);
        }
        let mut width = 0.0_f32;
        let mut strike: Option<(f32, f32)> = None;
        let mut underline: Option<(f32, f32)> = None;
        for run in self.buffer.layout_runs() {
            let baseline = y + run.line_y;
            // 每個字形畫完把它那份字距累加到後面所有字形的 x 上
            let mut tracked = 0.0_f32;
            for glyph in run.glyphs {
                let physical = glyph.physical((x + tracked, y), 1.0);
                tracked += tracking_px(&self.font_system, &mut self.tracking, glyph.font_id, style);
                let Some(image) = self
                    .cache
                    .get_image(&mut self.font_system, physical.cache_key)
                else {
                    continue;
                };
                let gx = physical.x + image.placement.left;
                let gy = run.line_y.round() as i32 + physical.y - image.placement.top;
                let (w, h) = (image.placement.width, image.placement.height);
                match image.content {
                    SwashContent::Mask => {
                        let table = gamma_table(&mut self.gamma_tables, style.gamma);
                        let data: Vec<u8> = image.data.iter().map(|&c| table[c as usize]).collect();
                        canvas.blend_mask(gx, gy, w, h, &data, style.color);
                    }
                    SwashContent::Color => canvas.blend_rgba(gx, gy, w, h, &image.data),
                    // 沒有申請亞畫素格式，不會出現
                    SwashContent::SubpixelMask => {}
                }
            }
            width = width.max(run.line_w + tracked);
            if style.strike {
                strike = Some((baseline, run.line_w + tracked));
            }
            if style.underline {
                underline = Some((baseline, run.line_w + tracked));
            }
        }
        if let Some((baseline, line_w)) = strike {
            // 刪除線穿過小寫字母中部
            let thickness = (style.size / 14.0).max(1.0);
            let line_y = baseline - style.size * 0.3;
            canvas.fill_rect(x, line_y, line_w, thickness, style.color);
        }
        if let Some((baseline, line_w)) = underline {
            // 下劃線壓在基線下面一點，與刪除線同一套粗細
            let thickness = (style.size / 14.0).max(1.0);
            let line_y = baseline + style.size * 0.14;
            canvas.fill_rect(x, line_y, line_w, thickness, style.color);
        }
        width
    }

    /// 描邊：整段文字的字形輪廓拼成一條路徑，按兩倍寬描一次（字形隨後壓在上面，露出來的就是向外的寬度）。
    /// 一條路徑描一次，半透明的描邊在字形相交處不會疊深。沒有輪廓的字形（點陣圖 emoji）不描。
    fn draw_stroke(
        &mut self,
        canvas: &mut Canvas,
        style: &TextStyle,
        x: f32,
        y: f32,
        width: f32,
        color: crate::color::Color,
    ) {
        let mut path = tiny_skia::PathBuilder::new();
        for run in self.buffer.layout_runs() {
            let baseline = y + run.line_y.round();
            let mut tracked = 0.0_f32;
            for glyph in run.glyphs {
                let physical = glyph.physical((x + tracked, y), 1.0);
                let origin_x = x + tracked + glyph.x + glyph.font_size * glyph.x_offset;
                let origin_y = baseline + glyph.y - glyph.font_size * glyph.y_offset;
                tracked += tracking_px(&self.font_system, &mut self.tracking, glyph.font_id, style);
                let Some(commands) = self
                    .cache
                    .get_outline_commands(&mut self.font_system, physical.cache_key)
                else {
                    continue;
                };
                for command in commands {
                    let at = |p: zeno::Point| (origin_x + p.x, origin_y - p.y);
                    match *command {
                        zeno::Command::MoveTo(p) => {
                            let (px, py) = at(p);
                            path.move_to(px, py);
                        }
                        zeno::Command::LineTo(p) => {
                            let (px, py) = at(p);
                            path.line_to(px, py);
                        }
                        zeno::Command::QuadTo(c, p) => {
                            let ((cx, cy), (px, py)) = (at(c), at(p));
                            path.quad_to(cx, cy, px, py);
                        }
                        zeno::Command::CurveTo(c1, c2, p) => {
                            let ((ax, ay), (bx, by), (px, py)) = (at(c1), at(c2), at(p));
                            path.cubic_to(ax, ay, bx, by, px, py);
                        }
                        zeno::Command::Close => path.close(),
                    }
                }
            }
        }
        if let Some(path) = path.finish() {
            canvas.stroke_outline(&path, width * 2.0, color);
        }
    }

    /// 每個字形用的字族名（相鄰相同的合併），拿來核對中日字形與 emoji 回退到了哪家字型。
    pub(crate) fn trace_families(&mut self, text: &str, style: &TextStyle) -> Vec<String> {
        self.shape(text, style);
        let mut names: Vec<String> = Vec::new();
        for run in self.buffer.layout_runs() {
            for glyph in run.glyphs {
                let name = self
                    .font_system
                    .db()
                    .face(glyph.font_id)
                    .and_then(|face| face.families.first().map(|(n, _)| n.clone()))
                    .unwrap_or_else(|| "?".to_owned());
                if names.last() != Some(&name) {
                    names.push(name);
                }
            }
        }
        names
    }

    fn shape(&mut self, text: &str, style: &TextStyle) {
        let family = match self.resolved.get(usize::from(style.family.0)) {
            Some(Some(name)) => Family::Name(name),
            _ => UI_FAMILY,
        };
        let attrs = Attrs::new()
            .family(family)
            .weight(cosmic_text::Weight(style.weight.0))
            .style(if style.italic {
                Style::Italic
            } else {
                Style::Normal
            })
            .color(style.color.to_cosmic());
        self.buffer
            .set_metrics(Metrics::new(style.size, style.line_height));
        self.buffer.set_size(None, None);
        self.buffer.set_text(text, &attrs, Shaping::Advanced, None);
        self.buffer.shape_until_scroll(&mut self.font_system, false);
    }
}

/// 檔案能解析就載入進字型庫；返回是否載入了。
fn load_font(font_system: &mut FontSystem, path: &Path) -> bool {
    match font_system.db_mut().load_font_file(path) {
        Ok(()) => {
            tracing::debug!(path = %path.display(), "加载主题字体");
            true
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "主题字体读不进来");
            false
        }
    }
}

/// 某張字型在這個字號下每個字形要加的間距（畫素）；第一次用到時解析它的 `trak` 表。
fn tracking_px(
    font_system: &FontSystem,
    cache: &mut HashMap<ID, Option<Trak>>,
    font_id: ID,
    style: &TextStyle,
) -> f32 {
    let trak = cache.entry(font_id).or_insert_with(|| {
        font_system
            .db()
            .with_face_data(font_id, Trak::parse)
            .flatten()
    });
    trak.as_ref()
        .map_or(0.0, |trak| trak.tracking_em(style.points) * style.size)
}

/// 覆蓋率 → gamma 校正後的覆蓋率。
fn gamma_table(cache: &mut HashMap<u32, Box<[u8; 256]>>, gamma: f32) -> &[u8; 256] {
    cache.entry(gamma.to_bits()).or_insert_with(|| {
        let mut table = [0u8; 256];
        for (i, out) in table.iter_mut().enumerate() {
            *out = ((i as f32 / 255.0).powf(gamma) * 255.0).round() as u8;
        }
        Box::new(table)
    })
}
