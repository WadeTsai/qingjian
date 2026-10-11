//! 主題裡的 SVG 圖片：載入主題時解析一次，畫的時候按要的畫素尺寸柵格成點陣圖（快取幾種尺寸），之後與 PNG 走同一條貼圖路徑。
//!
//! SVG 的單位當作點。不讀任何外部檔案（`<image href="…">` 的路徑一律不解析），內嵌點陣圖不解碼，`<text>` 不畫：
//! 主題的點陣圖另用 PNG，文字匯出前轉成輪廓。

use std::sync::{Arc, Mutex};

use resvg::usvg;
use tiny_skia::{Pixmap, Transform};

/// 柵格結果最多留幾種尺寸：動畫中途尺寸會變，舊的按先進先出丟掉。
const RASTER_CACHE: usize = 8;

/// 柵格過的尺寸（畫素寬高）與點陣圖，新的在後。
type Rasters = Vec<((u32, u32), Arc<Pixmap>)>;

pub(crate) struct SvgImage {
    tree: usvg::Tree,

    rasters: Mutex<Rasters>,
}

impl SvgImage {
    pub(crate) fn parse(data: &[u8]) -> Result<Self, usvg::Error> {
        let options = usvg::Options {
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_string: Box::new(|_, _| None),
                ..usvg::ImageHrefResolver::default()
            },
            ..usvg::Options::default()
        };
        Ok(Self {
            tree: usvg::Tree::from_data(data, &options)?,
            rasters: Mutex::new(Vec::new()),
        })
    }

    /// 原始尺寸（SVG 的寬高，當作點）。
    pub(crate) fn size(&self) -> (f32, f32) {
        let size = self.tree.size();
        (size.width(), size.height())
    }

    /// 整張拉伸到 `width` × `height` 畫素的點陣圖；尺寸為 0 時為 `None`。
    pub(crate) fn raster(&self, width: u32, height: u32) -> Option<Arc<Pixmap>> {
        let mut rasters = self
            .rasters
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some((_, pixmap)) = rasters.iter().find(|(size, _)| *size == (width, height)) {
            return Some(Arc::clone(pixmap));
        }
        let mut pixmap = Pixmap::new(width, height)?;
        let (svg_width, svg_height) = self.size();
        let transform = Transform::from_scale(width as f32 / svg_width, height as f32 / svg_height);
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());
        let pixmap = Arc::new(pixmap);
        if rasters.len() >= RASTER_CACHE {
            rasters.remove(0);
        }
        rasters.push(((width, height), Arc::clone(&pixmap)));
        Some(pixmap)
    }
}

impl std::fmt::Debug for SvgImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SvgImage")
            .field("size", &self.size())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::SvgImage;

    const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20">
        <rect width="10" height="20" fill="#ff0000"/>
    </svg>"##;

    #[test]
    fn rasters_at_any_size_and_caches() {
        let svg = SvgImage::parse(SQUARE.as_bytes()).unwrap();
        assert_eq!(svg.size(), (10.0, 20.0));
        let big = svg.raster(30, 60).unwrap();
        assert_eq!((big.width(), big.height()), (30, 60));
        let center = big.pixel(15, 30).unwrap();
        assert_eq!((center.red(), center.alpha()), (255, 255));
        assert!(std::sync::Arc::ptr_eq(&big, &svg.raster(30, 60).unwrap()));
        assert!(svg.raster(0, 10).is_none());
    }

    #[test]
    fn external_files_are_not_read() {
        // 引用磁碟上真實存在的紅色 SVG（SVG 不需要點陣圖解碼，讀了就畫得出來）：不讀，畫出來什麼都沒有
        let outside = std::env::temp_dir().join("qingjian-svg-outside.svg");
        std::fs::write(&outside, SQUARE).unwrap();
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="10" height="10">
                <image width="10" height="10" xlink:href="{}"/>
            </svg>"#,
            outside.display()
        );
        let svg = SvgImage::parse(source.as_bytes()).unwrap();
        let pixmap = svg.raster(10, 10).unwrap();
        assert!(pixmap.pixels().iter().all(|pixel| pixel.alpha() == 0));
    }

    #[test]
    fn broken_svg_is_an_error() {
        assert!(SvgImage::parse(b"<svg").is_err());
    }
}
