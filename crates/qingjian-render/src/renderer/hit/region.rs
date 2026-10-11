//! 一塊點選區域：內容區裡的畫素矩形加它對應的目標。

use super::HitTarget;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitRegion {
    /// 左上角在內容區裡的畫素座標。
    pub x: f32,

    pub y: f32,

    pub width: f32,

    pub height: f32,

    pub target: HitTarget,
}

impl HitRegion {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// 兩塊的外接矩形，目標取自己的。
    pub(crate) fn union(self, (x, y, width, height): (f32, f32, f32, f32)) -> Self {
        let left = self.x.min(x);
        let top = self.y.min(y);
        let right = (self.x + self.width).max(x + width);
        let bottom = (self.y + self.height).max(y + height);
        Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
            ..self
        }
    }
}
