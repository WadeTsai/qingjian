//! 懸浮狀態條（Windows）：幾格並排的小條 `[中 / 英][，。/ ,.][⚙]`，每格文字居中、格間一條細線，圓角背景加陰影。
//! macOS 用選單欄狀態項，沒有這一塊。主題沒寫 `status.root` 時排法固定，尺寸與顏色取 `status` 分節；寫了走節點樹（`tree.rs`）。

mod cell;
mod rendered;
mod tree;

pub use cell::StatusCell;
pub use rendered::RenderedStatus;

use taffy::{AlignItems, Dimension, Display, JustifyContent, NodeId, Position, Size, Style};

use super::Renderer;
use crate::error::RenderError;
use crate::frame::Mode;
use crate::scene::{Effect, Icon, Scene, Visual};
use crate::text::TextStyle;
use crate::theme::file::ColorSpec;
use crate::theme::file::node::EffectSpec;
use crate::theme::{TextSizes, Theme};

/// 一次渲染裡按倍數換算好的狀態條引數。
struct Metrics<'a> {
    theme: &'a Theme,

    scale: f32,

    /// 文字樣式（顏色後配）。
    font: crate::theme::FontSpec,
}

impl Metrics<'_> {
    fn px(&self, points: f32) -> f32 {
        points * self.scale
    }

    fn text_style(&self, emphasized: bool) -> TextStyle {
        let spec = &self.theme.file().status;
        let color = if emphasized {
            &spec.emphasized
        } else {
            &spec.normal
        };
        TextStyle::new(
            self.font.scaled(self.scale),
            self.font.size,
            self.theme.color(color),
            self.theme.text_gamma(),
        )
    }

    /// 狀態條沒有資料條件，條件顏色取 `else` 分支。
    fn effect(&self, spec: &EffectSpec) -> Effect {
        let color = match &spec.shadow().color {
            ColorSpec::Fixed(color)
            | ColorSpec::Switch {
                otherwise: color, ..
            } => color,
        };
        Effect::from_spec(spec, self.scale, self.theme.color(color))
    }
}

impl Renderer {
    /// 畫狀態條，返回點陣圖與各格右邊界（供點選命中）。`mode` 給節點樹畫法的主題顯示中 / 英等狀態。
    /// 設定裡改了候選字號時整條按同一比例放大縮小。
    /// 固定排法：每格寬 = 內容寬 + 兩側內邊距，高 = 行高 + 內邊距。
    pub fn render_status(
        &mut self,
        cells: &[StatusCell],
        mode: &Mode,
        theme: &Theme,
        scale: f32,
    ) -> Result<RenderedStatus, RenderError> {
        // 整條按候選字的比例縮放：去掉設定裡的字號再整體乘倍數，圖示、邊距、點選邊界跟文字一起變，與用哪個文字樣式無關
        let scale = scale * theme.candidate_scale();
        let theme = &theme.with_text_sizes(TextSizes::default());
        self.text.use_families(theme.families());
        let spec = &theme.file().status;
        if let Some(root) = &spec.root {
            return self.render_status_tree(root, cells, mode, theme, scale);
        }
        let m = Metrics {
            theme,
            scale,
            font: theme.font_ref(&spec.font),
        };
        let padding = m.px(spec.padding);
        let radius = m.px(spec.radius);
        let mut widths: Vec<f32> = cells
            .iter()
            .map(|cell| self.status_cell_width(cell, &m) + padding * 2.0)
            .collect();
        let total: f32 = widths.iter().sum();
        let content_width = total.ceil();
        // 取整多出來的零頭給最後一格，讓最後一格的右邊界正好是內容寬
        if let Some(last) = widths.last_mut() {
            *last += content_width - total;
        }
        let content_height = (m.px(m.font.line_height) + padding).ceil();

        let mut scene = Scene::new();
        let mut children = Vec::with_capacity(cells.len());
        for (i, (cell, width)) in cells.iter().zip(&widths).enumerate() {
            let size = (*width, content_height);
            children.push(status_cell(&mut scene, cell, &m, i > 0, size)?);
        }
        let root = scene.node(
            Style {
                display: Display::Flex,
                ..Style::default()
            },
            Visual::solid(theme.color(&spec.background), radius),
            &children,
        )?;
        let effects = spec.effects.iter().map(|effect| m.effect(effect)).collect();
        scene.set_effects(root, effects);
        scene.layout(root, &mut self.text)?;

        let edges = widths
            .iter()
            .scan(0.0, |x, width| {
                *x += width;
                Some(*x)
            })
            .collect();
        Ok(RenderedStatus {
            rendered: self.rasterize(&scene, root, (content_width, content_height), scale)?,
            cell_edges: edges,
        })
    }

    /// 一格內容的寬度（畫素，不含內邊距）。
    fn status_cell_width(&mut self, cell: &StatusCell, m: &Metrics) -> f32 {
        match cell {
            StatusCell::Text { text, emphasized } => {
                self.text.measure(text, &m.text_style(*emphasized)).width
            }
            StatusCell::Gear => m.px(m.theme.file().status.gear_size),
        }
    }
}

/// 一格：內容在格里居中；不是第一格時左邊畫一條上下各縮排半個內邊距的細線。
fn status_cell(
    scene: &mut Scene,
    cell: &StatusCell,
    m: &Metrics,
    separator: bool,
    (width, height): (f32, f32),
) -> Result<NodeId, RenderError> {
    let spec = &m.theme.file().status;
    let mut children = Vec::with_capacity(2);
    if separator {
        let inset = m.px(spec.padding) / 2.0;
        let style = Style {
            position: Position::Absolute,
            inset: taffy::Rect {
                left: taffy::LengthPercentageAuto::length(0.0),
                right: taffy::LengthPercentageAuto::auto(),
                top: taffy::LengthPercentageAuto::length(inset),
                bottom: taffy::LengthPercentageAuto::length(inset),
            },
            size: Size {
                width: Dimension::length(m.px(spec.separator_width)),
                height: Dimension::auto(),
            },
            ..Style::default()
        };
        let visual = Visual::solid(m.theme.color(&spec.separator), 0.0);
        children.push(scene.node(style, visual, &[])?);
    }
    let (content_style, content) = match cell {
        StatusCell::Text { text, emphasized } => (
            Style::default(),
            Visual::Text {
                text: text.clone(),
                style: m.text_style(*emphasized),
            },
        ),
        StatusCell::Gear => {
            let size = m.px(spec.gear_size);
            let style = Style {
                size: Size {
                    width: Dimension::length(size),
                    height: Dimension::length(size),
                },
                ..Style::default()
            };
            let visual = Visual::Icon {
                icon: Icon::Gear,
                size,
                color: m.theme.color(&spec.gear),
            };
            (style, visual)
        }
    };
    children.push(scene.node(content_style, content, &[])?);
    let style = Style {
        display: Display::Flex,
        justify_content: Some(JustifyContent::CENTER),
        align_items: Some(AlignItems::CENTER),
        size: Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        },
        ..Style::default()
    };
    scene.node(style, Visual::Group, &children)
}

#[cfg(test)]
mod tests {
    use super::StatusCell;
    use crate::fonts::FontLibrary;
    use crate::frame::Mode;
    use crate::renderer::Renderer;
    use crate::theme::Theme;

    #[test]
    fn cells_have_increasing_edges_ending_at_content_width() {
        // 沒有系統字型的環境（CI 容器）跳過
        let Ok(library) = FontLibrary::system("zh-CN") else {
            return;
        };
        let mut renderer = Renderer::new(library);
        let cells = [
            StatusCell::text("中 · 小鹤", true),
            StatusCell::text(",.", false),
            StatusCell::Gear,
        ];
        let out = renderer
            .render_status(&cells, &Mode::default(), &Theme::light(), 2.0)
            .unwrap();
        assert_eq!(out.cell_edges.len(), 3);
        assert!(out.cell_edges.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            out.cell_edges.last().map(|edge| edge.round() as u32),
            Some(out.rendered.content_width)
        );
        assert!(out.rendered.pixmap.width() > out.rendered.content_width);
        assert!(out.rendered.content_x > 0);
    }
}
