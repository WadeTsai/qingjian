//! 節點效果：`"effects": [{ "type": "drop-shadow", "x": 0, "y": 6, "blur": 12, "spread": 0, "color": "#0000005a" }]`，
//! 另有 `inner-shadow`。陰影的形狀取節點自己畫出來的 alpha（圓角框、九宮格圖片、文字都行），不含子節點。

use serde::Deserialize;

use crate::theme::file::ColorSpec;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum EffectSpec {
    /// 投影：墊在節點底下。
    DropShadow(ShadowSpec),

    /// 內陰影：畫在節點填充之上、子節點之下，只落在節點形狀裡。
    InnerShadow(ShadowSpec),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct ShadowSpec {
    /// 偏移（點），向右、向下為正。
    #[serde(default)]
    pub(crate) x: f32,

    #[serde(default)]
    pub(crate) y: f32,

    /// 模糊（點），與 CSS 的模糊半徑大致相當。
    #[serde(default)]
    pub(crate) blur: f32,

    /// 擴充套件（點）：投影把形狀往外擴、內陰影往裡縮；只對框生效。
    #[serde(default)]
    pub(crate) spread: f32,

    pub(crate) color: ColorSpec,
}

impl EffectSpec {
    pub(crate) fn shadow(&self) -> &ShadowSpec {
        match self {
            Self::DropShadow(shadow) | Self::InnerShadow(shadow) => shadow,
        }
    }
}
