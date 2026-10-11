//! 渲染結果：點陣圖加內容區（陰影邊之外的那塊）的位置。

use tiny_skia::Pixmap;

use super::{HitRegion, HitTarget};

pub struct Rendered {
    /// 預乘 RGBA 點陣圖，含陰影邊。
    pub pixmap: Pixmap,

    /// 內容區左上角在位圖裡的畫素座標。
    pub content_x: u32,

    pub content_y: u32,

    /// 內容區畫素寬高（視窗該有的大小）。
    pub content_width: u32,

    pub content_height: u32,

    /// 渲染用的倍數，殼把畫素換回點用。
    pub scale: f32,

    /// 還有動畫在播：隔這麼久調 [`crate::Renderer::tick`] 要下一幀；`None` 表示畫完了，不用再要。
    pub next_frame: Option<std::time::Duration>,

    /// 候選窗口裡可點的區域，譯詞在前（先命中）；狀態條為空，它按 [`crate::RenderedStatus::cell_edges`] 分格。
    pub hits: Vec<HitRegion>,
}

impl Rendered {
    /// 內容區裡一點（點，左上為原點）點中了什麼。
    pub fn hit_at(&self, x: f32, y: f32) -> Option<HitTarget> {
        let (x, y) = (x * self.scale, y * self.scale);
        self.hits
            .iter()
            .find(|region| region.contains(x, y))
            .map(|region| region.target)
    }

    /// 內容區寬高換回點。
    pub fn content_size_points(&self) -> (f32, f32) {
        (
            self.content_width as f32 / self.scale,
            self.content_height as f32 / self.scale,
        )
    }
}
