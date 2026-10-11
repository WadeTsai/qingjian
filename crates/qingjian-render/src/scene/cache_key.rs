//! 畫面快取的鍵：哪個節點的哪一份（第幾個效果的遮罩，或節點自己的畫面）、盒子在畫布裡的位置大小、畫布大小。
//! 都沒變就直接複用：動畫幀裡不動的節點不再重畫形狀、重做模糊、重新取樣圖片。

use taffy::NodeId;

use crate::canvas::Canvas;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct CacheKey {
    node: NodeId,

    slot: Slot,

    /// 盒子 `(x, y, 寬, 高)` 的位模式（畫素，浮點原樣比）。
    rect: [u32; 4],

    canvas: (u32, u32),
}

/// 快取的是節點的哪一份。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Slot {
    /// 第幾個效果的遮罩。
    Effect(usize),

    /// 節點自己的畫面（圖片填充的框）。
    Visual,

    /// 帶迴圈動畫的節點整棵子樹（未變換），每幀按姿態變換後貼上。
    Layer,
}

impl CacheKey {
    pub(super) fn new(
        node: NodeId,
        slot: Slot,
        rect: (f32, f32, f32, f32),
        canvas: &Canvas,
    ) -> Self {
        Self::with_size(node, slot, rect, (canvas.width(), canvas.height()))
    }

    /// 畫布大小直接給（往區域性小圖上畫、但要與整張畫布共用快取時）。
    pub(super) fn with_size(
        node: NodeId,
        slot: Slot,
        rect: (f32, f32, f32, f32),
        canvas: (u32, u32),
    ) -> Self {
        Self {
            node,
            slot,
            rect: [
                rect.0.to_bits(),
                rect.1.to_bits(),
                rect.2.to_bits(),
                rect.3.to_bits(),
            ],
            canvas,
        }
    }

    pub(super) fn node(&self) -> NodeId {
        self.node
    }
}
