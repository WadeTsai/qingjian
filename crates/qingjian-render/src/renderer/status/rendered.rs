//! 狀態條的渲染結果：點陣圖加各格的右邊界。

use crate::renderer::Rendered;

pub struct RenderedStatus {
    /// 點陣圖與內容區位置。
    pub rendered: Rendered,

    /// 各格右邊界在內容區裡的畫素 x，從左到右；點選按 x 落進哪格。
    pub cell_edges: Vec<f32>,
}
