//! 迴圈動畫的區域性重畫：不動的內容按畫序切成幾段各快取一張圖層，每幀只在動畫節點上一幀與這一幀佔的區域裡
//! 按原來的先後把「段 0、動畫節點 1、段 1、……」疊一遍，寫回上一幀的整張點陣圖。開銷只跟動的那一小塊有多大有關。
//!
//! 只在沒有過渡在播、所有動畫節點都「獨立」（見 `Scene::isolated`）時走這條路，否則整張重畫。

use std::time::{Duration, Instant};

use tiny_skia::Pixmap;

use super::retained::Retained;
use super::{Rendered, Renderer};
use crate::animation::{LOOP_INTERVAL, Pose};
use crate::canvas::Canvas;
use crate::error::RenderError;
use crate::scene::{LayerPlace, SplitMark};

/// 區域性重畫要的快取。
pub(super) struct Partial {
    /// 不動內容的各段（整張畫布大小），比動畫節點多一段。
    segments: Vec<Pixmap>,

    /// 動畫節點（按畫序），與段交替疊放。
    marks: Vec<SplitMark>,

    /// 上一幀的整張點陣圖。
    frame: Pixmap,

    /// 各動畫節點上一幀佔的整畫素矩形（`left, top, right, bottom`）。
    bounds: Vec<Option<(i32, i32, i32, i32)>>,
}

impl Renderer {
    /// 按區域性重畫出 `now` 這一幀；這一幀不適合區域性重畫（有過渡、動畫節點不獨立）時返回 `None`，呼叫方整張重畫。
    pub(super) fn paint_partial(
        &mut self,
        retained: &mut Retained,
        now: Instant,
    ) -> Result<Option<Rendered>, RenderError> {
        if !retained.transitions.is_empty()
            || retained.animated.is_empty()
            || !retained
                .animated
                .iter()
                .all(|(node, _)| retained.scene.isolated(*node))
        {
            retained.partial = None;
            return Ok(None);
        }
        let elapsed = self
            .clock
            .map_or(Duration::ZERO, |clock| now.saturating_duration_since(clock));
        let size = retained.size;
        if retained.partial.is_none() {
            retained.partial = Some(self.build_partial(retained)?);
        }
        let Some(partial) = retained.partial.as_mut() else {
            return Ok(None);
        };
        let poses: Vec<Pose> = partial
            .marks
            .iter()
            .map(|mark| {
                retained
                    .animated
                    .iter()
                    .find(|(node, _)| *node == mark.node)
                    .map_or(Pose::REST, |(_, keyframes)| keyframes.pose(elapsed))
            })
            .collect();
        let mut damage = Vec::new();
        for (index, (mark, pose)) in partial.marks.iter().zip(&poses).enumerate() {
            let place = LayerPlace {
                parent: mark.parent,
                size,
            };
            let now_bounds =
                retained
                    .scene
                    .posed_bounds(mark.node, &mut self.text, place, *pose)?;
            let union = match (partial.bounds[index], now_bounds) {
                (Some(a), Some(b)) => {
                    Some((a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
                }
                (a, b) => a.or(b),
            };
            damage.extend(union);
            partial.bounds[index] = now_bounds;
        }
        let mut frame = Canvas::from_pixmap(partial.frame.clone());
        for (left, top, right, bottom) in damage {
            let (left, top) = (left.max(0), top.max(0));
            let (right, bottom) = (right.min(size.0 as i32), bottom.min(size.1 as i32));
            if right <= left || bottom <= top {
                continue;
            }
            let mut patch = Canvas::new((right - left) as u32, (bottom - top) as u32)?;
            for (index, segment) in partial.segments.iter().enumerate() {
                patch.composite(-left, -top, segment);
                if let (Some(mark), Some(pose)) = (partial.marks.get(index), poses.get(index)) {
                    let place = LayerPlace {
                        parent: mark.parent,
                        size,
                    };
                    retained.scene.draw_posed(
                        mark.node,
                        &mut patch,
                        &mut self.text,
                        place,
                        *pose,
                        (left as f32, top as f32),
                    )?;
                }
            }
            frame.replace(left, top, &patch.into_pixmap());
        }
        let pixmap = frame.into_pixmap();
        partial.frame = pixmap.clone();
        let (x, y) = retained.origin;
        Ok(Some(Rendered {
            pixmap,
            content_x: x as u32,
            content_y: y as u32,
            content_width: retained.content.0 as u32,
            content_height: retained.content.1 as u32,
            scale: retained.scale,
            next_frame: self.loops_running(retained, now).then_some(LOOP_INTERVAL),
            hits: retained.hits.clone(),
        }))
    }

    /// 切段：先按分段模式走一遍記下動畫節點的畫序，再把每段畫進一張圖層。
    fn build_partial(&mut self, retained: &Retained) -> Result<Partial, RenderError> {
        let size = retained.size;
        let mut probe = Canvas::new(1, 1)?;
        let marks = retained.scene.paint_range(
            retained.root,
            &mut probe,
            &mut self.text,
            retained.origin,
            0..0,
        )?;
        let mut segments = Vec::with_capacity(marks.len() + 1);
        for index in 0..=marks.len() {
            let start = if index == 0 {
                0
            } else {
                marks[index - 1].order + 1
            };
            let end = marks.get(index).map_or(usize::MAX, |mark| mark.order);
            let mut canvas = Canvas::new(size.0, size.1)?;
            retained.scene.paint_range(
                retained.root,
                &mut canvas,
                &mut self.text,
                retained.origin,
                start..end,
            )?;
            segments.push(canvas.into_pixmap());
        }
        // 第一幀區域性重畫把整張畫布當作變了的區域，按段整張疊一遍
        let whole = Some((0, 0, size.0 as i32, size.1 as i32));
        let partial = Partial {
            bounds: vec![whole; marks.len()],
            segments,
            marks,
            frame: Canvas::new(size.0, size.1)?.into_pixmap(),
        };
        Ok(partial)
    }
}
