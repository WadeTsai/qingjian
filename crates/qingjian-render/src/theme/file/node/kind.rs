//! 節點種類（JSON 裡的 `type`）與各自的屬性。

use serde::Deserialize;

use super::NodeSpec;
use super::border::BorderSpec;
use super::caret::CaretSpec;
use super::direction::Direction;
use super::fill::FillSpec;
use super::table::TableSpec;
use super::tone_filter::ToneFilter;
use crate::scene::Icon;
use crate::theme::file::{ColorSpec, FontRef};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum NodeKind {
    /// 框：按方向排子節點（或排成表格），可以有填充、邊框與圓角。
    Frame {
        #[serde(default)]
        direction: Direction,

        fill: Option<FillSpec>,

        border: Option<BorderSpec>,

        #[serde(default)]
        radius: f32,

        table: Option<TableSpec>,

        #[serde(default)]
        children: Vec<NodeSpec>,
    },

    /// 一行文字：`bind` 繫結資料，或 `text` 寫死。
    Text {
        bind: Option<String>,

        text: Option<String>,

        /// `text.styles` 裡的樣式名，或在樣式上改字號 / 字重。
        font: FontRef,

        color: ColorSpec,

        /// 描邊。
        stroke: Option<BorderSpec>,
    },

    /// 向量圖示，在盒子裡垂直居中。
    Icon {
        icon: Icon,

        size: f32,

        color: ColorSpec,
    },

    /// 拼音行：各段按狀態著色，游標畫在游標位置。
    Preedit {
        font: FontRef,

        /// 已敲的拼音。
        typed: ColorSpec,

        /// 游標後未確認的拼音。
        rest: ColorSpec,

        /// 糾錯改掉的字母，帶刪除線。
        struck: ColorSpec,

        caret: CaretSpec,

        /// 拼音的描邊。
        stroke: Option<BorderSpec>,
    },

    /// 一串譯文片段，按深淺著色。
    Annotation {
        bind: String,

        font: FontRef,

        /// 譯文。
        gloss: ColorSpec,

        /// 生詞的譯文。
        fresh: ColorSpec,

        /// 淡色片段（假名注音）；詞性與分隔符沒單獨寫顏色時也用它。
        faint: ColorSpec,

        /// 詞性；不寫用 `faint`。
        pos: Option<ColorSpec>,

        /// 義項間的分隔；不寫用 `faint`。
        separator: Option<ColorSpec>,

        /// 只畫這幾種片段（詞性、譯文分兩行排時各寫一個節點）；不寫全畫。
        tones: Option<Vec<ToneFilter>>,

        /// 只畫前幾個義項（按分隔數）；不寫全畫。
        senses: Option<usize>,

        /// 描邊。
        stroke: Option<BorderSpec>,
    },

    /// 引用一個元件；這裡寫的盒子屬性蓋過元件根節點的。
    Use { component: String },

    /// 按列表資料把元件重複多份，每份繫結一項。
    Repeat { bind: String, component: String },
}
