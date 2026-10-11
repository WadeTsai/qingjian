//! 渲染器留住的上一幀：場景樹、畫布幾何、各配對節點的位置與在播的過渡。下一幀拿它找過渡起點，動畫幀拿它重畫。

use std::collections::HashMap;
use std::time::Instant;

use taffy::NodeId;

use crate::animation::{Keyframes, Placement, Transition};
use crate::layout::Layout;
use crate::scene::Scene;

use super::HitRegion;
use super::partial::Partial;

pub(super) struct Retained {
    pub(super) scene: Scene,

    pub(super) root: NodeId,

    /// 畫這一幀用的排布與倍數；變了就不跟新幀配對（位置不可比）。
    pub(super) layout: Layout,

    pub(super) scale: f32,

    /// 點陣圖寬高（畫素），動畫期間不變。
    pub(super) size: (u32, u32),

    /// 根節點左上角在位圖裡的位置（畫素，整數）。
    pub(super) origin: (f32, f32),

    /// 內容區寬高（畫素，已取整）。
    pub(super) content: (f32, f32),

    /// 帶過渡的節點佈局給的位置（相對根節點），按配對名。
    pub(super) placements: HashMap<String, Placement>,

    /// 在播的過渡。
    pub(super) transitions: Vec<Transition>,

    /// 帶迴圈動畫的節點與關鍵幀。
    pub(super) animated: Vec<(NodeId, Keyframes)>,

    /// 迴圈動畫區域性重畫的快取；整張重畫後作廢。
    pub(super) partial: Option<Partial>,

    /// 候選與譯詞的點選區域，動畫幀照抄（佈局不變）。
    pub(super) hits: Vec<HitRegion>,
}

impl Retained {
    /// `now` 時刻螢幕上各配對節點的實際位置：在播的取插值，其餘取佈局位置。新一幀的過渡從這裡出發，
    /// 所以過渡沒播完又來一幀時從半路接著走，不會跳回起點。
    pub(super) fn shown(&self, now: Instant) -> HashMap<String, Placement> {
        let mut shown = self.placements.clone();
        for transition in &self.transitions {
            shown.insert(transition.key.clone(), transition.at(now));
        }
        shown
    }
}
