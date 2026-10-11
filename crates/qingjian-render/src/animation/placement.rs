//! 節點在窗口裡的位置、大小（畫素，相對根節點左上角）與不透明度：過渡的起點、終點與中間值。

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placement {
    pub(crate) x: f32,

    pub(crate) y: f32,

    pub(crate) width: f32,

    pub(crate) height: f32,

    pub(crate) opacity: f32,
}

impl Placement {
    /// 從 `self` 到 `to` 走了 `progress`（可略超出 0–1，回彈曲線）。
    pub(crate) fn lerp(self, to: Self, progress: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * progress;
        Self {
            x: mix(self.x, to.x),
            y: mix(self.y, to.y),
            width: mix(self.width, to.width).max(0.0),
            height: mix(self.height, to.height).max(0.0),
            opacity: mix(self.opacity, to.opacity).clamp(0.0, 1.0),
        }
    }
}
