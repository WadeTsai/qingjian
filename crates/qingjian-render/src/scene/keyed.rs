//! 過渡用的節點登記：帶配對名的節點、它們在窗口裡的位置，以及動畫中蓋過佈局的位置。

use std::collections::HashMap;
use std::time::Duration;

use taffy::NodeId;

use super::{Scene, Visual};
use crate::animation::{Easing, Placement};
use crate::error::RenderError;

/// 一個帶過渡的節點：配對名（同一幀重複的已加序號）、過渡引數、佈局給的位置（相對根節點）。
pub(crate) struct Keyed {
    pub(crate) node: NodeId,

    pub(crate) key: String,

    pub(crate) duration: Duration,

    pub(crate) easing: Easing,

    pub(crate) placement: Placement,
}

impl Scene {
    /// 節點要過渡：配對名與引數。
    pub(crate) fn set_transition(
        &mut self,
        node: NodeId,
        key: String,
        duration: Duration,
        easing: Easing,
    ) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.transition = Some((key, duration, easing));
        }
    }

    /// 動畫中的位置（畫布畫素）；`None` 回到佈局算的。
    /// 自己不畫東西的上級容器拿子節點當陰影形狀，它們快取的遮罩跟著作廢；自己有畫面的上級不受影響。
    pub(crate) fn set_placed(&mut self, node: NodeId, placed: Option<Placement>) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.placed = placed;
        }
        let mut ancestors = Vec::new();
        let mut current = self.tree.parent(node);
        while let Some(parent) = current {
            if matches!(
                self.tree
                    .get_node_context(parent)
                    .map(|context| &context.visual),
                Some(Visual::Group)
            ) {
                ancestors.push(parent);
            }
            current = self.tree.parent(parent);
        }
        self.effect_cache
            .get_mut()
            .retain(|key, _| !ancestors.contains(&key.node()));
    }

    /// 以 `root` 為根、帶過渡的節點，按樹序；同一幀裡同名出現多次的按出現順序加 `#序號`。要在佈局之後調。
    pub(crate) fn keyed(&self, root: NodeId) -> Result<Vec<Keyed>, RenderError> {
        let mut found = Vec::new();
        self.collect(root, 0.0, 0.0, &mut found)?;
        let mut counts: HashMap<String, usize> = HashMap::new();
        for keyed in &found {
            *counts.entry(keyed.key.clone()).or_default() += 1;
        }
        let mut seen: HashMap<String, usize> = HashMap::new();
        for keyed in &mut found {
            if counts[&keyed.key] > 1 {
                let index = seen.entry(keyed.key.clone()).or_default();
                keyed.key = format!("{}#{index}", keyed.key);
                *index += 1;
            }
        }
        Ok(found)
    }

    fn collect(
        &self,
        node: NodeId,
        x: f32,
        y: f32,
        found: &mut Vec<Keyed>,
    ) -> Result<(), RenderError> {
        let layout = self.tree.layout(node)?;
        let (x, y) = (x + layout.location.x, y + layout.location.y);
        let context = self.tree.get_node_context(node);
        let own = context.map_or(1.0, |context| context.opacity);
        if let Some((key, duration, easing)) =
            context.and_then(|context| context.transition.clone())
        {
            found.push(Keyed {
                node,
                key,
                duration,
                easing,
                placement: Placement {
                    x,
                    y,
                    width: layout.size.width,
                    height: layout.size.height,
                    opacity: own,
                },
            });
        }
        for child in self.tree.children(node)? {
            self.collect(child, x, y, found)?;
        }
        Ok(())
    }
}
