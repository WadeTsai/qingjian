//! 場景圖：一棵 Taffy 佈局樹，每個節點帶一個 [`Visual`]。建樹 → 佈局（文字按實際整形量寬）→ 按樹序畫。
//!
//! 節點的位置全由佈局給出：流內節點按 flex / grid 排，高亮條、游標這類疊在別的節點上的用絕對定位。
//! 單位是畫素（主題的點數在建樹時已乘倍數），佈局不取整，保證與直接算座標的結果一致。

mod animated;
mod baseline;
mod box_paint;
mod cache_key;
mod draw;
mod effect;
mod extent;
mod fill;
mod icon;
mod keyed;
mod layer_place;
mod locate;
mod node;
mod paint;
mod split;
mod visual;

use std::cell::RefCell;
use std::collections::HashMap;

use taffy::prelude::TaffyMaxContent;
use taffy::{AlignItems, AvailableSpace, GridPlacement, Line, NodeId, Size, Style, TaffyTree};

use crate::error::RenderError;
use crate::text::{TextPainter, TextSize};
use cache_key::CacheKey;
use draw::EffectMask;
use tiny_skia::Pixmap;

pub(crate) use box_paint::BoxPaint;
pub(crate) use effect::{Effect, EffectKind};
pub(crate) use fill::Fill;
pub(crate) use icon::Icon;
pub(crate) use keyed::Keyed;
pub(crate) use layer_place::LayerPlace;
pub(crate) use node::SceneNode;
pub(crate) use split::SplitMark;
pub(crate) use visual::Visual;

pub(crate) struct Scene {
    /// 佈局樹，節點上下文是它畫什麼。
    tree: TaffyTree<SceneNode>,

    /// 算過的效果遮罩：動畫幀重畫時沒動的節點直接複用（離屏圖層裡畫的子樹也用同一份，鍵裡有畫布大小）。
    effect_cache: RefCell<HashMap<CacheKey, Option<EffectMask>>>,

    /// 畫好的圖片填充框（九宮格視窗背景這類，每畫素都要取樣）：動畫幀裡沒動就貼回去。
    visual_cache: RefCell<HashMap<CacheKey, CachedVisual>>,

    /// 分段模式：只畫畫序編號落在這個範圍裡的節點（見 `split.rs`）；`None` 為正常畫。
    split: RefCell<Option<std::ops::Range<usize>>>,

    /// 分段模式下下一個節點的編號。
    order: std::cell::Cell<usize>,

    /// 分段模式下跳過的動畫節點。
    marks: RefCell<Vec<SplitMark>>,

    /// 表格的各行（不含橫跨整行的格子），佈局時按基線對齊。
    table_rows: Vec<Vec<NodeId>>,
}

/// 快取的一塊畫面：左上角在畫布裡的整數位置與點陣圖。
struct CachedVisual {
    left: i32,

    top: i32,

    pixmap: Pixmap,
}

impl Scene {
    pub(crate) fn new() -> Self {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        Self {
            tree,
            effect_cache: RefCell::new(HashMap::new()),
            visual_cache: RefCell::new(HashMap::new()),
            split: RefCell::new(None),
            order: std::cell::Cell::new(0),
            marks: RefCell::new(Vec::new()),
            table_rows: Vec::new(),
        }
    }

    /// 加一個節點，`children` 按畫的先後排。
    pub(crate) fn node(
        &mut self,
        style: Style,
        visual: Visual,
        children: &[NodeId],
    ) -> Result<NodeId, RenderError> {
        let node = self.tree.new_with_children(style, children)?;
        self.tree.set_node_context(
            node,
            Some(SceneNode {
                visual,
                opacity: 1.0,
                effects: Vec::new(),
                transition: None,
                placed: None,
                animation: None,
                pose: None,
            }),
        )?;
        Ok(node)
    }

    /// 節點連同子節點的不透明度。
    pub(crate) fn set_opacity(&mut self, node: NodeId, opacity: f32) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.opacity = opacity.clamp(0.0, 1.0);
        }
    }

    /// 節點的投影、內陰影。
    pub(crate) fn set_effects(&mut self, node: NodeId, effects: Vec<Effect>) {
        if let Some(context) = self.tree.get_node_context_mut(node) {
            context.effects = effects;
        }
    }

    /// 把表格裡的格子放到指定的行、列；`stretch` 時撐滿所佔的格子（橫跨整行的高亮條）。
    pub(crate) fn place(
        &mut self,
        node: NodeId,
        row: Line<GridPlacement>,
        column: Line<GridPlacement>,
        stretch: bool,
    ) -> Result<(), RenderError> {
        let mut style = self.tree.style(node)?.clone();
        style.grid_row = row;
        style.grid_column = column;
        if stretch {
            style.justify_self = Some(AlignItems::STRETCH);
            style.align_self = Some(AlignItems::STRETCH);
        }
        self.tree.set_style(node, style)?;
        Ok(())
    }

    /// 以 `root` 為根按內容撐開算佈局，返回根節點的寬高。表格行裡的格子基線沒對齊時補上邊距再算一遍。
    pub(crate) fn layout(
        &mut self,
        root: NodeId,
        text: &mut TextPainter,
    ) -> Result<(f32, f32), RenderError> {
        // 佈局會對同一個葉子按不同約束量好幾次；單行文字的尺寸與約束無關，量一次記下來
        let mut measured: HashMap<NodeId, TextSize> = HashMap::new();
        self.compute(root, text, &mut measured)?;
        if self.align_baselines(&measured)? {
            self.compute(root, text, &mut measured)?;
        }
        let size = self.tree.layout(root)?.size;
        Ok((size.width, size.height))
    }

    fn compute(
        &mut self,
        root: NodeId,
        text: &mut TextPainter,
        measured: &mut HashMap<NodeId, TextSize>,
    ) -> Result<(), RenderError> {
        self.tree.compute_layout_with_measure(
            root,
            Size::MAX_CONTENT,
            |inputs, node, visual, style| {
                taffy::compute_leaf_layout(
                    inputs,
                    style,
                    |_, _| 0.0,
                    |known, _available: Size<AvailableSpace>| match visual {
                        Some(SceneNode {
                            visual:
                                Visual::Text {
                                    text: content,
                                    style,
                                },
                            ..
                        }) => {
                            let size = *measured
                                .entry(node)
                                .or_insert_with(|| text.measure(content, style));
                            Size {
                                width: known.width.unwrap_or(size.width),
                                height: known.height.unwrap_or(size.height),
                            }
                        }
                        _ => Size {
                            width: known.width.unwrap_or(0.0),
                            height: known.height.unwrap_or(0.0),
                        },
                    },
                )
            },
        )?;
        Ok(())
    }
}
