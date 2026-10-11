//! 節點效果（畫素、顏色已按外觀取好）：投影墊在節點底下，內陰影壓在節點填充上。

use crate::color::Color;
use crate::theme::file::node::EffectSpec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Effect {
    pub(crate) kind: EffectKind,

    /// 偏移。
    pub(crate) x: f32,

    pub(crate) y: f32,

    pub(crate) blur: f32,

    /// 擴充套件：投影外擴、內陰影內縮，只對盒子生效。
    pub(crate) spread: f32,

    pub(crate) color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectKind {
    DropShadow,
    InnerShadow,
}

impl Effect {
    /// 主題裡的效果按倍數換成畫素，顏色由呼叫方按資料與外觀取好。
    pub(crate) fn from_spec(spec: &EffectSpec, scale: f32, color: Color) -> Self {
        let shadow = spec.shadow();
        Self {
            kind: match spec {
                EffectSpec::DropShadow(_) => EffectKind::DropShadow,
                EffectSpec::InnerShadow(_) => EffectKind::InnerShadow,
            },
            x: shadow.x * scale,
            y: shadow.y * scale,
            blur: shadow.blur * scale,
            spread: shadow.spread * scale,
            color,
        }
    }

    /// 投影伸出節點盒子的最遠距離；視窗根節點按它在四周留邊。
    pub(crate) fn overhang(&self) -> f32 {
        match self.kind {
            EffectKind::DropShadow => {
                crate::shadow::reach(self.blur) + self.spread + self.x.abs().max(self.y.abs())
            }
            EffectKind::InnerShadow => 0.0,
        }
    }
}
