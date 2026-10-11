//! 主題裡的節點：種類 + 顯示條件 + 盒子屬性。

mod align;
mod animation;
mod border;
mod box_spec;
mod caret;
mod direction;
mod edges;
mod effect;
mod fill;
mod kind;
mod length;
mod position;
mod span;
mod table;
mod tone_filter;
mod transition;

use serde::Deserialize;

pub(crate) use align::Align;
pub(crate) use animation::{AnimationSpec, KeyframeSpec};
pub(crate) use border::BorderSpec;
pub(crate) use box_spec::BoxSpec;
pub(crate) use direction::Direction;
pub(crate) use edges::Edges;
pub(crate) use effect::EffectSpec;
pub(crate) use fill::{FillSpec, StopSpec};
pub(crate) use kind::NodeKind;
pub(crate) use length::Length;
pub(crate) use position::Position;
pub(crate) use span::Span;
pub(crate) use tone_filter::ToneFilter;
pub(crate) use transition::TransitionSpec;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NodeSpec {
    #[serde(flatten)]
    pub(crate) kind: NodeKind,

    /// 顯示條件：資料欄位名，`!` 取反，`a|b` 任一成立。不寫總是顯示。
    pub(crate) when: Option<String>,

    /// 過渡配對用的名字：新一幀與上一幀同 `id` 的節點之間插值。
    pub(crate) id: Option<String>,

    /// 寫了才過渡，沒寫的節點即使有 `id` 也直接跳到新位置。
    pub(crate) transition: Option<TransitionSpec>,

    /// 迴圈動畫（關鍵幀）。
    pub(crate) animation: Option<AnimationSpec>,

    #[serde(flatten)]
    pub(crate) layout: BoxSpec,
}
