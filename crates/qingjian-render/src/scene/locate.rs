//! 節點在根節點座標裡的矩形：狀態條按各格的盒子分點選區域。

use taffy::NodeId;

use super::Scene;
use crate::error::RenderError;

impl Scene {
    /// `node` 在 `root` 座標裡的矩形（左、上、寬、高，畫素），佈局之後調。
    pub(crate) fn rect_in(
        &self,
        node: NodeId,
        root: NodeId,
    ) -> Result<(f32, f32, f32, f32), RenderError> {
        let (x, y) = self.parent_origin(node, root)?;
        let layout = self.tree.layout(node)?;
        Ok((
            x + layout.location.x,
            y + layout.location.y,
            layout.size.width,
            layout.size.height,
        ))
    }
}
