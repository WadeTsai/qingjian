//! 節點的盒子：外邊距、內邊距、尺寸、定位、對齊。都可以不寫；`use` 引用元件時寫的會蓋過元件根節點上的。

use serde::Deserialize;

use super::align::Align;
use super::edges::Edges;
use super::effect::EffectSpec;
use super::position::Position;
use super::span::Span;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(crate) struct BoxSpec {
    pub(crate) margin: Option<Edges>,

    pub(crate) padding: Option<Edges>,

    /// 子節點之間的間距（點）。
    pub(crate) gap: Option<f32>,

    pub(crate) width: Option<f32>,

    pub(crate) height: Option<f32>,

    pub(crate) min_width: Option<f32>,

    pub(crate) position: Option<Position>,

    /// 絕對定位時四邊到父框的距離，`[上, 右, 下, 左]`。
    pub(crate) inset: Option<Edges>,

    pub(crate) align_self: Option<Align>,

    /// 表格裡跨列。
    pub(crate) span: Option<Span>,

    /// 不透明度（0–1），作用於節點連同子節點。
    pub(crate) opacity: Option<f32>,

    /// 投影、內陰影。
    pub(crate) effects: Option<Vec<EffectSpec>>,
}

impl BoxSpec {
    /// `over` 裡寫了的項蓋過自己的。
    pub(crate) fn overridden_by(&self, over: &Self) -> Self {
        Self {
            margin: over.margin.or(self.margin),
            padding: over.padding.or(self.padding),
            gap: over.gap.or(self.gap),
            width: over.width.or(self.width),
            height: over.height.or(self.height),
            min_width: over.min_width.or(self.min_width),
            position: over.position.or(self.position),
            inset: over.inset.or(self.inset),
            align_self: over.align_self.or(self.align_self),
            span: over.span.or(self.span),
            opacity: over.opacity.or(self.opacity),
            effects: over.effects.clone().or_else(|| self.effects.clone()),
        }
    }
}
