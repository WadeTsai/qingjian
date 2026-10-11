//! 分段畫：動畫節點把樹序切成幾段，每段不動的內容各畫進一張圖層快取起來；迴圈動畫的每一幀只在變了的那塊區域裡
//! 按樹序把「段 0、動畫節點 1、段 1、……」疊一遍，只重畫這一塊。
//!
//! 節點按畫的先後編號（進 `paint_subtree` 時取號），分段模式下只畫號在範圍裡的節點；動畫節點只佔一個號、整棵子樹跳過，
//! 另記下它的號與父節點左上角。容器拿子節點當陰影形狀時臨時退出分段模式，免得打亂編號。

use std::ops::Range;

use taffy::NodeId;

use super::Scene;
use crate::canvas::Canvas;
use crate::error::RenderError;
use crate::text::TextPainter;

/// 分段裡跳過的一個動畫節點：畫的先後編號、父節點左上角（畫布畫素）。
#[derive(Debug, Clone, Copy)]
pub(crate) struct SplitMark {
    pub(crate) node: NodeId,

    pub(crate) order: usize,

    pub(crate) parent: (f32, f32),
}

impl Scene {
    /// 按分段模式畫一遍 `root`：只畫編號落在 `range` 裡的節點，返回途中跳過的動畫節點。
    pub(crate) fn paint_range(
        &self,
        root: NodeId,
        canvas: &mut Canvas,
        text: &mut TextPainter,
        origin: (f32, f32),
        range: Range<usize>,
    ) -> Result<Vec<SplitMark>, RenderError> {
        self.split.replace(Some(range));
        self.order.set(0);
        self.marks.borrow_mut().clear();
        let painted = self.paint(root, canvas, text, origin.0, origin.1);
        self.split.replace(None);
        painted?;
        Ok(self.marks.borrow_mut().drain(..).collect())
    }

    /// 分段模式下進一個節點：取號，返回這個節點自己要不要畫。不在分段模式時總是畫。
    pub(super) fn take_order(&self) -> bool {
        let range = self.split.borrow().clone();
        match range {
            Some(range) => {
                let order = self.order.get();
                self.order.set(order + 1);
                range.contains(&order)
            }
            None => true,
        }
    }

    /// 分段模式下遇到動畫節點：佔一個號、記下來，整棵子樹跳過。返回是否跳過了。
    pub(super) fn skip_animated(&self, node: NodeId, parent: (f32, f32)) -> bool {
        if self.split.borrow().is_none() {
            return false;
        }
        let animated = self
            .tree
            .get_node_context(node)
            .is_some_and(|context| context.animation.is_some());
        if animated {
            let order = self.order.get();
            self.order.set(order + 1);
            self.marks.borrow_mut().push(SplitMark {
                node,
                order,
                parent,
            });
        }
        animated
    }

    /// 暫時退出分段模式跑 `f`（拿子節點當陰影形狀時用），跑完恢復。
    pub(super) fn unsplit<T>(&self, f: impl FnOnce() -> T) -> T {
        let saved = self.split.replace(None);
        let order = self.order.get();
        let result = f();
        self.split.replace(saved);
        self.order.set(order);
        result
    }

    /// 動畫節點能不能走區域性重畫：它的上級裡沒有動畫節點、半透明容器、拿子節點當陰影形狀的容器
    /// （這些上級的畫面隨它變），自己也不在過渡中。
    pub(crate) fn isolated(&self, node: NodeId) -> bool {
        let mut current = self.tree.parent(node);
        while let Some(parent) = current {
            let Some(context) = self.tree.get_node_context(parent) else {
                return false;
            };
            let group_with_effects =
                matches!(context.visual, super::Visual::Group) && !context.effects.is_empty();
            if context.animation.is_some() || context.opacity < 1.0 || group_with_effects {
                return false;
            }
            current = self.tree.parent(parent);
        }
        self.tree
            .get_node_context(node)
            .is_some_and(|context| context.transition.is_none())
    }
}
