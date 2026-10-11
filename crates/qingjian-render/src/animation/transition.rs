//! 一個在播的過渡：哪個節點、從哪到哪、什麼時候開始、多久、什麼曲線。

use std::time::{Duration, Instant};

use taffy::NodeId;

use super::{Easing, Placement};

#[derive(Debug, Clone)]
pub(crate) struct Transition {
    /// 場景樹裡的節點（這一幀的）。
    pub(crate) node: NodeId,

    /// 配對用的 `id`（重複時帶序號）。
    pub(crate) key: String,

    pub(crate) from: Placement,

    pub(crate) to: Placement,

    pub(crate) start: Instant,

    pub(crate) duration: Duration,

    pub(crate) easing: Easing,
}

impl Transition {
    /// `now` 時刻的位置。
    pub(crate) fn at(&self, now: Instant) -> Placement {
        self.from
            .lerp(self.to, self.easing.apply(self.progress(now)))
    }

    /// 時間進度 0–1。
    fn progress(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (now.saturating_duration_since(self.start).as_secs_f32() / self.duration.as_secs_f32())
            .min(1.0)
    }

    pub(crate) fn finished(&self, now: Instant) -> bool {
        self.progress(now) >= 1.0
    }
}
