//! 迴圈動畫的節點：登記關鍵幀、每幀設姿態，以及變換後的節點畫出來佔哪一塊（算畫出範圍、區域性重畫用）。

use taffy::NodeId;

use super::Scene;
use super::extent::Extent;
use crate::animation::{Keyframes, Pose};
use crate::error::RenderError;

impl Scene {
    pub(crate) fn set_animation(&mut self, node: NodeId, keyframes: Keyframes) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.animation = Some(keyframes);
        }
    }

    /// 這一幀的姿態；`None` 或靜止姿態時按原樣畫。
    pub(crate) fn set_pose(&mut self, node: NodeId, pose: Option<Pose>) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.pose = pose.filter(|pose| *pose != Pose::REST);
        }
    }

    /// 以 `root` 為根、帶迴圈動畫的節點與關鍵幀，按樹序。
    pub(crate) fn animated(&self, root: NodeId) -> Result<Vec<(NodeId, Keyframes)>, RenderError> {
        let mut found = Vec::new();
        self.collect_animated(root, &mut found)?;
        Ok(found)
    }

    fn collect_animated(
        &self,
        node: NodeId,
        found: &mut Vec<(NodeId, Keyframes)>,
    ) -> Result<(), RenderError> {
        if let Some(keyframes) = self
            .tree
            .get_node_context(node)
            .and_then(|context| context.animation.clone())
        {
            found.push((node, keyframes));
        }
        for child in self.tree.children(node)? {
            self.collect_animated(child, found)?;
        }
        Ok(())
    }

    /// 節點連同子樹畫出來的範圍（畫布畫素），父節點左上角在 `(x, y)`；含投影、描邊伸出的部分。
    pub(crate) fn subtree_extent(
        &self,
        node: NodeId,
        x: f32,
        y: f32,
    ) -> Result<Extent, RenderError> {
        let layout = self.tree.layout(node)?;
        let (nx, ny) = (x + layout.location.x, y + layout.location.y);
        let mut extent = Extent {
            left: nx,
            top: ny,
            right: nx + layout.size.width,
            bottom: ny + layout.size.height,
        };
        self.grow_from(node, x, y, &mut extent)?;
        Ok(extent)
    }

    /// 節點盒子的中心（畫布畫素），父節點左上角在 `(x, y)`：姿態繞它旋轉縮放。
    pub(crate) fn center(&self, node: NodeId, x: f32, y: f32) -> Result<(f32, f32), RenderError> {
        let layout = self.tree.layout(node)?;
        Ok((
            x + layout.location.x + layout.size.width / 2.0,
            y + layout.location.y + layout.size.height / 2.0,
        ))
    }

    /// 節點父節點左上角相對 `root` 左上角的位置（畫素，佈局給的）。
    pub(crate) fn parent_origin(
        &self,
        node: NodeId,
        root: NodeId,
    ) -> Result<(f32, f32), RenderError> {
        let mut origin = (0.0, 0.0);
        let mut current = self.tree.parent(node);
        while let Some(parent) = current {
            if parent == root {
                break;
            }
            let location = self.tree.layout(parent)?.location;
            origin.0 += location.x;
            origin.1 += location.y;
            current = self.tree.parent(parent);
        }
        Ok(origin)
    }
}
