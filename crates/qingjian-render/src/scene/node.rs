//! 場景樹節點的上下文：畫什麼、效果、整棵子樹的不透明度，以及過渡用的配對名與動畫中的位置。

use super::effect::Effect;
use super::visual::Visual;
use crate::animation::{Easing, Keyframes, Placement, Pose};

#[derive(Debug, Clone)]
pub(crate) struct SceneNode {
    pub(crate) visual: Visual,

    /// 不透明度（0–1），小於 1 時子樹先畫到離屏圖層再合成。
    pub(crate) opacity: f32,

    /// 投影、內陰影，按寫的順序畫。
    pub(crate) effects: Vec<Effect>,

    /// 過渡配對名（主題裡的 `id`，重複時帶序號）與過渡引數；沒寫 `transition` 的為 `None`。
    pub(crate) transition: Option<(String, std::time::Duration, Easing)>,

    /// 動畫中的位置（畫布畫素）與不透明度，蓋過佈局算的；不在動畫中為 `None`。
    pub(crate) placed: Option<Placement>,

    /// 迴圈動畫的關鍵幀。
    pub(crate) animation: Option<Keyframes>,

    /// 這一幀的姿態（迴圈動畫算出來的），整棵子樹跟著變換；`None` 為原樣。
    pub(crate) pose: Option<Pose>,
}
