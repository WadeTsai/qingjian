//! 渲染器的錯誤。

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// 介面字型一個都沒載入到，沒法畫字。
    #[error("no UI font could be loaded (tried {tried:?})")]
    NoUiFont { tried: Vec<PathBuf> },

    /// 點陣圖尺寸為零或大到 tiny-skia 拒絕分配。
    #[error("invalid bitmap size {width}x{height}")]
    InvalidSize { width: u32, height: u32 },

    /// 佈局樹出錯（節點 id 失效），屬於渲染器自身的 bug。
    #[error("layout failed: {0}")]
    Layout(#[from] taffy::TaffyError),
}
