//! 過渡：新一幀按配對名找上一幀同名節點，位置不同就從舊位置插值過去。殼按 [`Rendered::next_frame`] 定時調 [`Renderer::tick_at`]。

use std::time::{Duration, Instant};

use super::retained::Retained;
use super::{Rendered, Renderer};
use crate::animation::{FRAME_INTERVAL, LOOP_INTERVAL, Placement, Transition};
use crate::canvas::Canvas;
use crate::error::RenderError;
use crate::scene::Keyed;

impl Renderer {
    /// 跟隨系統的「減少動態效果」：開著時過渡直接跳到終點。
    pub fn set_reduce_motion(&mut self, reduce: bool) {
        self.reduce_motion = reduce;
    }

    /// 不播動畫：系統減少動態效果，或設定裡關了。
    pub(super) fn still(&self) -> bool {
        self.reduce_motion || self.animations_off
    }

    /// 忘掉上一幀（視窗隱藏時調）：下次顯示不從舊位置過渡，迴圈動畫從頭播。
    pub fn forget(&mut self) {
        self.last = None;
        self.clock = None;
    }

    /// 有過渡在播時，按 `now` 重畫一幀（不重建樹、不排版）；沒有在播的返回 `None`。
    pub fn tick_at(&mut self, now: Instant) -> Result<Option<Rendered>, RenderError> {
        let Some(mut retained) = self.last.take() else {
            return Ok(None);
        };
        if retained.transitions.is_empty() && !self.loops_running(&retained, now) {
            self.last = Some(retained);
            return Ok(None);
        }
        let rendered = match self.paint_partial(&mut retained, now) {
            Ok(Some(rendered)) => Ok(rendered),
            Ok(None) => self.paint_retained(&mut retained, now),
            Err(error) => Err(error),
        };
        self.last = Some(retained);
        rendered.map(Some)
    }

    /// 同 [`Self::tick_at`]，用當前時間。
    pub fn tick(&mut self) -> Result<Option<Rendered>, RenderError> {
        self.tick_at(Instant::now())
    }

    /// 與上一幀配對，算出要播的過渡。
    pub(super) fn transitions(
        &self,
        keyed: &[Keyed],
        layout: crate::layout::Layout,
        scale: f32,
        now: Instant,
    ) -> Vec<Transition> {
        if self.still() {
            return Vec::new();
        }
        let Some(last) = self
            .last
            .as_ref()
            .filter(|last| last.layout == layout && last.scale == scale)
        else {
            return Vec::new();
        };
        let shown = last.shown(now);
        keyed
            .iter()
            .filter_map(|keyed| {
                let from = *shown.get(&keyed.key)?;
                (from != keyed.placement && !keyed.duration.is_zero()).then(|| Transition {
                    node: keyed.node,
                    key: keyed.key.clone(),
                    from,
                    to: keyed.placement,
                    start: now,
                    duration: keyed.duration,
                    easing: keyed.easing,
                })
            })
            .collect()
    }

    /// 按 `now` 把過渡中的節點擺到插值位置，畫整張；播完的過渡清掉（最後一幀就是佈局位置）。
    pub(super) fn paint_retained(
        &mut self,
        retained: &mut Retained,
        now: Instant,
    ) -> Result<Rendered, RenderError> {
        let (x, y) = retained.origin;
        retained.partial = None;
        for transition in &retained.transitions {
            let placed = (!transition.finished(now)).then(|| {
                let at = transition.at(now);
                Placement {
                    x: at.x + x,
                    y: at.y + y,
                    ..at
                }
            });
            retained.scene.set_placed(transition.node, placed);
        }
        retained
            .transitions
            .retain(|transition| !transition.finished(now));
        let elapsed = self
            .clock
            .map_or(Duration::ZERO, |clock| now.saturating_duration_since(clock));
        for (node, keyframes) in &retained.animated {
            // 減少動態效果：停在開頭那一幀
            let at = if self.still() {
                Duration::ZERO
            } else {
                elapsed
            };
            retained.scene.set_pose(*node, Some(keyframes.pose(at)));
        }
        let mut canvas = Canvas::new(retained.size.0, retained.size.1)?;
        retained
            .scene
            .paint(retained.root, &mut canvas, &mut self.text, x, y)?;
        Ok(Rendered {
            pixmap: canvas.into_pixmap(),
            content_x: x as u32,
            content_y: y as u32,
            content_width: retained.content.0 as u32,
            content_height: retained.content.1 as u32,
            scale: retained.scale,
            next_frame: if !retained.transitions.is_empty() {
                Some(FRAME_INTERVAL)
            } else if self.loops_running(retained, now) {
                Some(LOOP_INTERVAL)
            } else {
                None
            },
            hits: retained.hits.clone(),
        })
    }

    /// 還有迴圈動畫要接著播（減少動態效果時不播）。
    pub(super) fn loops_running(&self, retained: &Retained, now: Instant) -> bool {
        if self.still() {
            return false;
        }
        let elapsed = self
            .clock
            .map_or(Duration::ZERO, |clock| now.saturating_duration_since(clock));
        retained
            .animated
            .iter()
            .any(|(_, keyframes)| !keyframes.finished(elapsed))
    }
}
