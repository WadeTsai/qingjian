//! 場景畫出來會佔多大：各節點的盒子（自己畫東西的才算）加投影伸出的部分、文字描邊，並在一起。
//! 伸出視窗的裝飾（負 `inset` 的絕對定位節點）靠它把點陣圖撐大，視窗本體仍是根節點的盒子。

use taffy::NodeId;

use super::{Effect, Scene, SceneNode, Visual};
use crate::error::RenderError;

/// 相對根節點左上角的範圍（畫素），總是包含根節點的盒子。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Extent {
    pub(crate) left: f32,

    pub(crate) top: f32,

    pub(crate) right: f32,

    pub(crate) bottom: f32,
}

impl Scene {
    /// 以 `root` 為根的整棵樹畫出來的範圍；要在 [`Scene::layout`] 之後調。
    pub(crate) fn extent(&self, root: NodeId) -> Result<Extent, RenderError> {
        let size = self.tree.layout(root)?.size;
        let mut extent = Extent {
            left: 0.0,
            top: 0.0,
            right: size.width,
            bottom: size.height,
        };
        self.grow(root, 0.0, 0.0, &mut extent)?;
        Ok(extent)
    }

    /// 同 `grow`，給別的模組用。
    pub(super) fn grow_from(
        &self,
        node: NodeId,
        x: f32,
        y: f32,
        extent: &mut Extent,
    ) -> Result<(), RenderError> {
        self.grow(node, x, y, extent)
    }

    /// 把 `node`（父節點左上角在 `(x, y)`）及其子樹並進 `extent`。
    fn grow(&self, node: NodeId, x: f32, y: f32, extent: &mut Extent) -> Result<(), RenderError> {
        let layout = self.tree.layout(node)?;
        let (x, y) = (x + layout.location.x, y + layout.location.y);
        if let Some(reach) = self.reach(node) {
            extent.include((x, y, layout.size.width, layout.size.height), reach);
        }
        for child in self.tree.children(node)? {
            self.grow(child, x, y, extent)?;
        }
        Ok(())
    }

    /// 節點畫出來伸出盒子多遠（投影、文字描邊）；自己不畫東西又沒有效果的容器為 `None`。
    pub(crate) fn reach(&self, node: NodeId) -> Option<f32> {
        let SceneNode {
            visual, effects, ..
        } = self.tree.get_node_context(node)?;
        let shadow = effects.iter().map(Effect::overhang).fold(0.0, f32::max);
        let stroke = match visual {
            Visual::Text { style, .. } => style.stroke.map_or(0.0, |(width, _)| width),
            _ => 0.0,
        };
        (!matches!(visual, Visual::Group) || shadow > 0.0).then_some(shadow.max(stroke))
    }
}

impl Extent {
    /// 並進一個盒子（相對根節點）連同它伸出的 `reach`。
    pub(crate) fn include(&mut self, (x, y, width, height): (f32, f32, f32, f32), reach: f32) {
        self.left = self.left.min(x - reach);
        self.top = self.top.min(y - reach);
        self.right = self.right.max(x + width + reach);
        self.bottom = self.bottom.max(y + height + reach);
    }
}
