//! 節點的迴圈動畫：`"animation": { "duration": 2400, "loop": true, "easing": "ease-in-out", "keyframes": [...] }`。
//! 關鍵幀寫 `at`（0–1）與要動的屬性（`x` / `y` 平移點數、`rotate` 度、`scale`、`opacity`），變換作用於整棵子樹、不影響佈局。

use serde::Deserialize;

use crate::animation::Easing;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct AnimationSpec {
    /// 一輪的時長（毫秒）。
    pub(crate) duration: f32,

    /// 迴圈播放；`false` 時播一輪停在最後一幀。
    #[serde(default = "default_loop", rename = "loop")]
    pub(crate) repeat: bool,

    /// 相鄰兩個關鍵幀之間的緩動。
    #[serde(default)]
    pub(crate) easing: Easing,

    pub(crate) keyframes: Vec<KeyframeSpec>,
}

fn default_loop() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct KeyframeSpec {
    /// 在一輪裡的位置（0–1）。
    pub(crate) at: f32,

    pub(crate) x: Option<f32>,

    pub(crate) y: Option<f32>,

    /// 旋轉（度，順時針，繞節點中心）。
    pub(crate) rotate: Option<f32>,

    /// 縮放（繞節點中心）。
    pub(crate) scale: Option<f32>,

    pub(crate) opacity: Option<f32>,
}
