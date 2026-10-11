//! 例項化：主題模板 + 一幀資料 → 場景樹。節點按 `when` 取捨、按 `bind` 填資料，`use` 展開元件、`repeat` 按列表展開。
//!
//! 一個模板節點可能產出零個（條件不成立、繫結為空）或多個（`repeat`）場景節點，所以都往 `out` 裡推。

mod cell;
mod context;
mod frame;
mod layout_style;
mod leaf;
mod row;

use taffy::NodeId;

use crate::animation::Keyframes;
use crate::error::RenderError;
use crate::frame::Frame;
use crate::layout::Layout;
use crate::renderer::StatusCell;
use crate::scene::{Effect, Scene, Visual};
use crate::text::{TextPainter, TextStyle};
use crate::theme::Theme;
use crate::theme::file::node::{BorderSpec, BoxSpec, EffectSpec, NodeKind, NodeSpec};
use crate::theme::file::{ColorSpec, FontRef};

use context::Context;

pub(super) struct Builder<'a> {
    pub(super) scene: &'a mut Scene,

    pub(super) theme: &'a Theme,

    /// 點 → 畫素。
    pub(super) scale: f32,

    /// 量游標位置用。
    pub(super) text: &'a mut TextPainter,

    /// 狀態條各格的盒子，按格的順序；條件不成立沒畫出來的格為 `None`。
    pub(super) cell_nodes: Vec<Option<NodeId>>,

    /// 候選窗口裡各候選產出的節點（第幾個候選，節點）：點選區域取它們的外接矩形。
    pub(super) candidate_nodes: Vec<(usize, NodeId)>,

    /// 各候選譯文裡每條譯詞的片段節點（第幾個候選，第幾條，節點）。
    pub(super) sense_nodes: Vec<(usize, usize, NodeId)>,
}

impl Builder<'_> {
    /// 候選視窗的根節點。
    pub(super) fn window(&mut self, frame: &Frame, layout: Layout) -> Result<NodeId, RenderError> {
        let windows = &self.theme.file().windows;
        let spec = match layout {
            Layout::Vertical => &windows.vertical,
            Layout::Horizontal => &windows.horizontal,
        };
        self.root(spec, Context::new(frame))
    }

    /// 狀態條的根節點（主題寫了 `status.root`）：格子按 `cells` 展開，各格的盒子記在 [`Self::cell_nodes`]。
    pub(super) fn status(
        &mut self,
        spec: &NodeSpec,
        frame: &Frame,
        cells: &[StatusCell],
    ) -> Result<NodeId, RenderError> {
        self.root(spec, Context::status(frame, cells))
    }

    /// 例項化根模板；什麼都沒產出（條件不成立）時給一個空節點。
    fn root(&mut self, spec: &NodeSpec, ctx: Context) -> Result<NodeId, RenderError> {
        let mut out = Vec::with_capacity(1);
        self.node(spec, ctx, &BoxSpec::default(), &mut out)?;
        let root = match out.first() {
            Some(&root) => root,
            None => self
                .scene
                .node(taffy::Style::default(), Visual::Group, &[])?,
        };
        Ok(root)
    }

    /// 例項化一個模板節點；`over` 是外層 `use` 寫的盒子屬性，蓋過這個節點自己的。
    fn node(
        &mut self,
        spec: &NodeSpec,
        ctx: Context,
        over: &BoxSpec,
        out: &mut Vec<NodeId>,
    ) -> Result<(), RenderError> {
        if spec.when.as_deref().is_some_and(|when| !ctx.holds(when)) {
            return Ok(());
        }
        let layout = spec.layout.overridden_by(over);
        let theme = self.theme;
        match &spec.kind {
            NodeKind::Frame { .. } => {
                self.frame(spec, &layout, ctx, out)?;
                self.mark_transition(spec, out);
                Ok(())
            }
            NodeKind::Use { component } => match theme.file().components.get(component) {
                Some(component) => self.node(component, ctx, &layout, out),
                None => Ok(()),
            },
            NodeKind::Repeat { bind, component } if bind == "cells" => {
                let Some(component) = theme.file().components.get(component) else {
                    return Ok(());
                };
                let cells = ctx.cells;
                for (i, cell) in cells.iter().enumerate() {
                    let before = out.len();
                    self.node(component, ctx.with_cell(cell, i, cells.len()), &layout, out)?;
                    // 每格第一個產出的節點就是這一格的盒子，點選按它的右邊界分格
                    self.cell_nodes.push(out.get(before).copied());
                }
                Ok(())
            }
            NodeKind::Repeat { bind, component } => {
                let (Some(rows), Some(component)) =
                    (ctx.list(bind), theme.file().components.get(component))
                else {
                    return Ok(());
                };
                for (i, row) in rows.iter().enumerate() {
                    let row_ctx = ctx.with_row(row, i, rows.len());
                    let before = out.len();
                    self.node(component, row_ctx, &layout, out)?;
                    self.candidate_nodes
                        .extend(out[before..].iter().map(|&node| (i, node)));
                }
                Ok(())
            }
            _ => {
                if let Some(node) = self.leaf(spec, &layout, ctx)? {
                    out.push(node);
                    self.mark_transition(spec, out);
                }
                Ok(())
            }
        }
    }

    /// 節點寫了 `id` 與 `transition`：登記到剛產出的場景節點上（`out` 的最後一個）；寫了 `animation` 同樣登記。
    fn mark_transition(&mut self, spec: &NodeSpec, out: &[NodeId]) {
        if let (Some(animation), Some(&node)) = (&spec.animation, out.last()) {
            self.scene
                .set_animation(node, Keyframes::from_spec(animation, self.scale));
        }
        if let (Some(id), Some(transition), Some(&node)) = (&spec.id, &spec.transition, out.last())
        {
            let duration =
                std::time::Duration::from_secs_f32(transition.duration.max(0.0) / 1000.0);
            self.scene
                .set_transition(node, id.clone(), duration, transition.easing);
        }
    }

    /// 盒子屬性裡的不透明度（作用於整棵子樹）與效果設到節點上。
    fn apply_layer(&mut self, node: NodeId, layout: &BoxSpec, ctx: Context) {
        if let Some(opacity) = layout.opacity
            && opacity < 1.0
        {
            self.scene.set_opacity(node, opacity);
        }
        if let Some(effects) = &layout.effects {
            let effects = effects.iter().map(|spec| self.effect(spec, ctx)).collect();
            self.scene.set_effects(node, effects);
        }
    }

    /// 效果換成畫素、顏色按當前資料與外觀取值。
    fn effect(&self, spec: &EffectSpec, ctx: Context) -> Effect {
        Effect::from_spec(spec, self.scale, self.color(&spec.shadow().color, ctx))
    }

    /// 節點顏色：條件寫法按當前資料取分支，再按外觀取值。
    fn color(&self, spec: &ColorSpec, ctx: Context) -> crate::color::Color {
        let color = match spec {
            ColorSpec::Fixed(color) => color,
            ColorSpec::Switch {
                condition,
                then,
                otherwise,
            } => {
                if ctx.holds(condition) {
                    then
                } else {
                    otherwise
                }
            }
        };
        self.theme.color(color)
    }

    /// 命名文字樣式按倍數換成畫素、配上顏色與當前外觀的 gamma。
    /// 文字的描邊：寬度換成畫素、顏色按當前資料與外觀取值。
    fn stroke(
        &self,
        stroke: Option<&BorderSpec>,
        ctx: Context,
    ) -> Option<(f32, crate::color::Color)> {
        stroke.map(|stroke| (stroke.width * self.scale, self.color(&stroke.color, ctx)))
    }

    fn text_style(&self, font: &FontRef, color: crate::color::Color) -> TextStyle {
        let spec = self.theme.font_ref(font);
        TextStyle::new(
            spec.scaled(self.scale),
            spec.size,
            color,
            self.theme.text_gamma(),
        )
    }
}
