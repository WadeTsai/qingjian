//! 一張預覽圖的結果。

use std::path::PathBuf;

use super::ShotError;

#[derive(Debug)]
pub struct Shot {
    /// 檔名：`vertical.png` / `horizontal.png` / `vertical-dark.png`。
    pub file: &'static str,

    /// 寫出的路徑，或這張為什麼沒畫出來。
    pub result: Result<PathBuf, ShotError>,
}
