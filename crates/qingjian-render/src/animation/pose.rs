//! 迴圈動畫某一刻的姿態：平移（畫素）、旋轉（度）、縮放、不透明度，作用於節點整棵子樹。

use tiny_skia::Transform;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Pose {
    pub(crate) x: f32,

    pub(crate) y: f32,

    pub(crate) rotate: f32,

    pub(crate) scale: f32,

    pub(crate) opacity: f32,
}

impl Pose {
    pub(crate) const REST: Self = Self {
        x: 0.0,
        y: 0.0,
        rotate: 0.0,
        scale: 1.0,
        opacity: 1.0,
    };

    /// 繞 `(cx, cy)` 的變換：先縮放、旋轉，再平移。
    pub(crate) fn transform(&self, cx: f32, cy: f32) -> Transform {
        Transform::from_translate(-cx, -cy)
            .post_scale(self.scale, self.scale)
            .post_rotate(self.rotate)
            .post_translate(cx + self.x, cy + self.y)
    }
}
