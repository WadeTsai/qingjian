//! 按樹序畫：先畫節點自己（投影墊底、內陰影壓在填充上；自己不畫的容器取子節點當陰影形狀），再畫子節點；座標是父節點位置加布局給的相對位置。
//! 不透明度小於 1 的節點連同子樹先畫到離屏圖層，再按不透明度合成；帶迴圈動畫姿態的節點連同子樹畫進一塊快取的小圖，按姿態變換後貼上。

use taffy::NodeId;

use super::cache_key::{CacheKey, Slot};
use super::draw::{draw_shape, draw_visual, effect_mask};
use super::{CachedVisual, Fill, LayerPlace};
use super::{EffectKind, Scene, SceneNode, Visual};
use crate::canvas::Canvas;
use crate::error::RenderError;
use crate::text::TextPainter;

impl Scene {
    /// 把 `root` 畫到畫布上，`(x, y)` 是 `root` 父節點左上角在畫布裡的畫素座標（根節點就是它自己的左上角）。
    pub(crate) fn paint(
        &self,
        root: NodeId,
        canvas: &mut Canvas,
        text: &mut TextPainter,
        x: f32,
        y: f32,
    ) -> Result<(), RenderError> {
        if self.skip_animated(root, (x, y)) {
            return Ok(());
        }
        let pose = self.tree.get_node_context(root).and_then(|node| node.pose);
        match pose {
            Some(pose) => self.paint_posed(root, canvas, text, x, y, pose),
            None => self.paint_body(root, canvas, text, x, y),
        }
    }

    /// 帶姿態的節點：整棵子樹（原樣）畫進一塊離屏小圖並快取，按姿態繞節點中心變換後貼上。
    fn paint_posed(
        &self,
        node: NodeId,
        canvas: &mut Canvas,
        text: &mut TextPainter,
        x: f32,
        y: f32,
        pose: crate::animation::Pose,
    ) -> Result<(), RenderError> {
        let place = LayerPlace {
            parent: (x, y),
            size: (canvas.width(), canvas.height()),
        };
        self.draw_posed(node, canvas, text, place, pose, (0.0, 0.0))
    }

    /// 帶姿態的節點畫到 `canvas` 上；`offset` 是 `canvas` 左上角在整張畫布裡的位置（區域性重畫時往一塊小圖上畫）。
    pub(crate) fn draw_posed(
        &self,
        node: NodeId,
        canvas: &mut Canvas,
        text: &mut TextPainter,
        place: LayerPlace,
        pose: crate::animation::Pose,
        offset: (f32, f32),
    ) -> Result<(), RenderError> {
        let parent = place.parent;
        let Some((left, top)) = self.posed_layer(node, text, place)? else {
            return Ok(());
        };
        let (cx, cy) = self.center(node, parent.0, parent.1)?;
        let transform = pose
            .transform(cx, cy)
            .pre_translate(left as f32, top as f32)
            .post_translate(-offset.0, -offset.1);
        let key = self.layer_key(node, place)?;
        if let Some(cached) = self.visual_cache.borrow().get(&key) {
            canvas.draw_transformed(&cached.pixmap, transform, pose.opacity);
        }
        Ok(())
    }

    /// 帶姿態的節點這一幀畫出來佔的整畫素矩形（整張畫布座標，含抗鋸齒一畫素）：`(left, top, right, bottom)`。
    pub(crate) fn posed_bounds(
        &self,
        node: NodeId,
        text: &mut TextPainter,
        place: LayerPlace,
        pose: crate::animation::Pose,
    ) -> Result<Option<(i32, i32, i32, i32)>, RenderError> {
        let parent = place.parent;
        let Some((left, top)) = self.posed_layer(node, text, place)? else {
            return Ok(None);
        };
        let key = self.layer_key(node, place)?;
        let (width, height) = match self.visual_cache.borrow().get(&key) {
            Some(cached) => (cached.pixmap.width() as f32, cached.pixmap.height() as f32),
            None => return Ok(None),
        };
        let (cx, cy) = self.center(node, parent.0, parent.1)?;
        let transform = pose
            .transform(cx, cy)
            .pre_translate(left as f32, top as f32);
        let mut corners = [
            tiny_skia::Point::from_xy(0.0, 0.0),
            tiny_skia::Point::from_xy(width, 0.0),
            tiny_skia::Point::from_xy(0.0, height),
            tiny_skia::Point::from_xy(width, height),
        ];
        transform.map_points(&mut corners);
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for point in corners {
            x0 = x0.min(point.x);
            y0 = y0.min(point.y);
            x1 = x1.max(point.x);
            y1 = y1.max(point.y);
        }
        Ok(Some((
            x0.floor() as i32 - 1,
            y0.floor() as i32 - 1,
            x1.ceil() as i32 + 1,
            y1.ceil() as i32 + 1,
        )))
    }

    fn layer_key(&self, node: NodeId, place: LayerPlace) -> Result<CacheKey, RenderError> {
        let (parent, size) = (place.parent, place.size);
        let layout = self.tree.layout(node)?;
        let rect = (
            parent.0 + layout.location.x,
            parent.1 + layout.location.y,
            layout.size.width,
            layout.size.height,
        );
        Ok(CacheKey::with_size(node, Slot::Layer, rect, size))
    }

    /// 確保節點整棵子樹（原樣）已畫進快取小圖；返回小圖左上角在整張畫布裡的位置。
    fn posed_layer(
        &self,
        node: NodeId,
        text: &mut TextPainter,
        place: LayerPlace,
    ) -> Result<Option<(i32, i32)>, RenderError> {
        let parent = place.parent;
        let key = self.layer_key(node, place)?;
        if let Some(cached) = self.visual_cache.borrow().get(&key) {
            return Ok(Some((cached.left, cached.top)));
        }
        let extent = self.subtree_extent(node, parent.0, parent.1)?;
        let (left, top) = (extent.left.floor(), extent.top.floor());
        let layer_size = (
            (extent.right.ceil() - left).max(1.0) as u32,
            (extent.bottom.ceil() - top).max(1.0) as u32,
        );
        let mut layer = Canvas::new(layer_size.0, layer_size.1)?;
        self.unsplit(|| self.paint_body(node, &mut layer, text, parent.0 - left, parent.1 - top))?;
        self.visual_cache.borrow_mut().insert(
            key,
            CachedVisual {
                left: left as i32,
                top: top as i32,
                pixmap: layer.into_pixmap(),
            },
        );
        Ok(Some((left as i32, top as i32)))
    }

    /// 原樣畫（不管姿態）：不透明度小於 1 時先畫到離屏圖層。
    fn paint_body(
        &self,
        root: NodeId,
        canvas: &mut Canvas,
        text: &mut TextPainter,
        x: f32,
        y: f32,
    ) -> Result<(), RenderError> {
        let opacity = self.tree.get_node_context(root).map_or(1.0, |node| {
            node.placed.map_or(node.opacity, |placed| placed.opacity)
        });
        if opacity >= 1.0 {
            return self.paint_subtree(root, canvas, text, x, y);
        }
        if opacity <= 0.0 {
            return Ok(());
        }
        let mut layer = Canvas::new(canvas.width(), canvas.height())?;
        self.paint_subtree(root, &mut layer, text, x, y)?;
        canvas.draw_layer(&layer.into_pixmap(), opacity);
        Ok(())
    }

    fn paint_subtree(
        &self,
        root: NodeId,
        canvas: &mut Canvas,
        text: &mut TextPainter,
        x: f32,
        y: f32,
    ) -> Result<(), RenderError> {
        let layout = self.tree.layout(root)?;
        let placed = self
            .tree
            .get_node_context(root)
            .and_then(|node| node.placed);
        // 動畫中的節點按插值後的位置畫，子樹跟著走
        let rect = match placed {
            Some(placed) => (placed.x, placed.y, placed.width, placed.height),
            None => (
                x + layout.location.x,
                y + layout.location.y,
                layout.size.width,
                layout.size.height,
            ),
        };
        let (x, y) = (rect.0, rect.1);
        let own = self.take_order();
        if let Some(SceneNode {
            visual, effects, ..
        }) = self.tree.get_node_context(root).filter(|_| own)
        {
            let children = self.tree.children(root)?;
            for kind in [EffectKind::DropShadow, EffectKind::InnerShadow] {
                if kind == EffectKind::InnerShadow {
                    self.draw_own(canvas, text, root, visual, rect)?;
                }
                for (index, effect) in effects.iter().enumerate() {
                    if effect.kind != kind {
                        continue;
                    }
                    let key = CacheKey::new(root, Slot::Effect(index), rect, canvas);
                    let cached = self.effect_cache.borrow().get(&key).cloned();
                    let mask = match cached {
                        Some(mask) => mask,
                        None => {
                            // 自己不畫東西的容器（譯文、拼音行）拿子節點當形狀
                            let mut shape = |layer: &mut Canvas, (x, y, width, height), spread| {
                                match visual {
                                    Visual::Group => self.unsplit(|| {
                                        for &child in &children {
                                            self.paint(child, layer, text, x, y)?;
                                        }
                                        Ok::<(), RenderError>(())
                                    })?,
                                    _ => draw_shape(
                                        layer,
                                        text,
                                        visual,
                                        (x, y, width, height),
                                        spread,
                                    ),
                                }
                                Ok(())
                            };
                            let bounds = (canvas.width(), canvas.height());
                            let mask = effect_mask(bounds, rect, effect, &mut shape)?;
                            self.effect_cache.borrow_mut().insert(key, mask.clone());
                            mask
                        }
                    };
                    if let Some(mask) = mask {
                        canvas.blend_mask(
                            mask.left,
                            mask.top,
                            mask.width,
                            mask.height,
                            &mask.alpha,
                            effect.color,
                        );
                    }
                }
            }
        }
        for child in self.tree.children(root)? {
            self.paint(child, canvas, text, x, y)?;
        }
        Ok(())
    }

    /// 畫節點自己的畫面。圖片填充的框先畫進一塊離屏圖、按鍵快取，動畫幀裡沒動就直接貼回。
    fn draw_own(
        &self,
        canvas: &mut Canvas,
        text: &mut TextPainter,
        node: NodeId,
        visual: &Visual,
        rect: (f32, f32, f32, f32),
    ) -> Result<(), RenderError> {
        let is_image = matches!(visual, Visual::Box(paint)
            if matches!(paint.fill, Some(Fill::Image { .. } | Fill::Svg { .. })));
        if !is_image {
            draw_visual(canvas, text, visual, rect);
            return Ok(());
        }
        let key = CacheKey::new(node, Slot::Visual, rect, canvas);
        if !self.visual_cache.borrow().contains_key(&key) {
            let (x, y, width, height) = rect;
            let (left, top) = (x.floor(), y.floor());
            let size = (
                ((x + width).ceil() - left).max(1.0) as u32,
                ((y + height).ceil() - top).max(1.0) as u32,
            );
            let mut layer = Canvas::new(size.0, size.1)?;
            draw_visual(&mut layer, text, visual, (x - left, y - top, width, height));
            self.visual_cache.borrow_mut().insert(
                key,
                CachedVisual {
                    left: left as i32,
                    top: top as i32,
                    pixmap: layer.into_pixmap(),
                },
            );
        }
        if let Some(cached) = self.visual_cache.borrow().get(&key) {
            canvas.composite(cached.left, cached.top, &cached.pixmap);
        }
        Ok(())
    }
}
