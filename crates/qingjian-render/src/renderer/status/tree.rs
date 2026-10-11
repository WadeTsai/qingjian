//! 狀態條的節點樹畫法：主題寫了 `status.root` 時走這裡，與候選視窗同一套模板、資料與繪製。
//! 格子由 `repeat` 繫結 `cells` 展開；點選區域按各格盒子的右邊界分，格子之間夾著的裝飾算前一格。

use super::{RenderedStatus, StatusCell};
use crate::error::RenderError;
use crate::frame::{Frame, Mode};
use crate::renderer::Renderer;
use crate::renderer::build::Builder;
use crate::scene::Scene;
use crate::theme::Theme;
use crate::theme::file::node::NodeSpec;

impl Renderer {
    pub(super) fn render_status_tree(
        &mut self,
        spec: &NodeSpec,
        cells: &[StatusCell],
        mode: &Mode,
        theme: &Theme,
        scale: f32,
    ) -> Result<RenderedStatus, RenderError> {
        let frame = Frame {
            mode: mode.clone(),
            ..Frame::default()
        };
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
        let root = builder.status(spec, &frame, cells)?;
        let cell_nodes = std::mem::take(&mut builder.cell_nodes);
        let (content_width, content_height) = scene.layout(root, &mut self.text)?;
        // 沒畫出來的格右邊界同前一格（點不到它）；最後一格延到內容右邊
        let mut edges = Vec::with_capacity(cell_nodes.len());
        let mut previous = 0.0_f32;
        for node in cell_nodes {
            if let Some(node) = node {
                let (x, _, width, _) = scene.rect_in(node, root)?;
                previous = previous.max(x + width);
            }
            edges.push(previous);
        }
        if let Some(last) = edges.last_mut() {
            *last = last.max(content_width.ceil());
        }
        let content = (content_width.ceil(), content_height.ceil());
        Ok(RenderedStatus {
            rendered: self.rasterize(&scene, root, content, scale)?,
            cell_edges: edges,
        })
    }
}
