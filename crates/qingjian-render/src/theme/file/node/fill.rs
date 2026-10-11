//! 框的填充：純色（顏色寫法同 `color`，可帶條件）、線性 / 徑向漸變、圖片（拉伸或九宮格）。
//!
//! - 線性：`{ "linear": 90, "stops": ["@a", "@b"] }`，角度同 CSS（0 自下而上、90 自左而右），漸變線長到盒子對角。
//! - 徑向：`{ "radial": [0.5, 0.5], "stops": [...] }`，圓心按盒子寬高的比例，半徑到最遠的角。
//! - 色標：顏色（均分），或 `["@a", 0.3]` 帶位置（0–1）。
//! - 圖片：`{ "image": "images/bg.png", "slice": [24, 24, 24, 24], "scale": 2 }`，路徑相對 `theme.json`；
//!   `slice` 是九宮格四邊切進去的圖片畫素（上右下左），不寫就整張拉伸；`scale` 是一個點對幾個圖片畫素（2 倍圖寫 2），預設 1。

use serde::Deserialize;

use crate::theme::file::ColorSpec;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum FillSpec {
    Linear {
        /// 角度（度）。
        linear: f32,

        stops: Vec<StopSpec>,
    },

    Radial {
        /// 圓心，盒子寬高的比例。
        radial: [f32; 2],

        stops: Vec<StopSpec>,
    },

    Image {
        /// 相對 `theme.json` 的路徑。
        image: String,

        /// 九宮格切邊（圖片畫素，上右下左）。
        slice: Option<[f32; 4]>,

        /// 一個點對幾個圖片畫素。
        #[serde(default = "default_scale")]
        scale: f32,
    },

    Color(ColorSpec),
}

/// 漸變的一個色標。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum StopSpec {
    /// 帶位置（0–1）。
    At(ColorSpec, f32),

    /// 不帶位置，按順序均分。
    Even(ColorSpec),
}

fn default_scale() -> f32 {
    1.0
}

impl FillSpec {
    /// 用到的全部顏色（校驗用）。
    pub(crate) fn colors(&self) -> Vec<&ColorSpec> {
        match self {
            Self::Linear { stops, .. } | Self::Radial { stops, .. } => {
                stops.iter().map(StopSpec::color).collect()
            }
            Self::Image { .. } => Vec::new(),
            Self::Color(color) => vec![color],
        }
    }
}

impl StopSpec {
    pub(crate) fn color(&self) -> &ColorSpec {
        match self {
            Self::At(color, _) | Self::Even(color) => color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_fill_form() {
        let parse = |json: &str| serde_json::from_str::<FillSpec>(json).unwrap();
        assert!(matches!(parse(r#""@accent""#), FillSpec::Color(_)));
        assert!(matches!(
            parse(r#"{ "if": "highlighted", "then": "@a", "else": "@b" }"#),
            FillSpec::Color(ColorSpec::Switch { .. })
        ));
        let FillSpec::Linear { linear, stops } =
            parse(r##"{ "linear": 90, "stops": ["#ff0000", ["@b", 0.3]] }"##)
        else {
            panic!("not linear");
        };
        assert_eq!(linear, 90.0);
        assert!(matches!(stops[1], StopSpec::At(_, position) if position == 0.3));
        assert!(matches!(
            parse(r#"{ "radial": [0.5, 0.2], "stops": ["@a", "@b"] }"#),
            FillSpec::Radial { .. }
        ));
        let FillSpec::Image { slice, scale, .. } =
            parse(r#"{ "image": "images/bg.png", "slice": [8, 8, 8, 8] }"#)
        else {
            panic!("not image");
        };
        assert_eq!((slice, scale), (Some([8.0; 4]), 1.0));
    }
}
