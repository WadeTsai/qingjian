//! 主題的圖片與字型素材：載入主題時讀進來，圖片解碼一次、渲染時按路徑取；字型只核對路徑，由渲染器載入。
//! 使用者主題從主題目錄讀；內建主題的圖片編程序序（`include_bytes!`），按同樣的路徑查表，不帶字型（只用系統字型）。
//!
//! 路徑相對 `theme.json`，不能是絕對路徑、不能含 `..`（主題不能讀主題目錄以外的檔案）；圖片認 PNG（邊長上限 [`MAX_SIDE`]）
//! 與 SVG（`.svg` 結尾，檔案上限 [`MAX_SVG_BYTES`]、原始尺寸不超過 [`MAX_SIDE`]）；
//! 字型單個檔案上限 [`MAX_FONT_BYTES`]。讀不進來的記警告，用到圖片的填充不畫，用到字型的樣式按回退鏈往後找。

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use tiny_skia::Pixmap;

use super::SvgImage;
use super::file::ThemeFile;
use super::file::node::{FillSpec, NodeKind, NodeSpec};

/// 圖片邊長上限（畫素）。
const MAX_SIDE: u32 = 4096;

/// 單個 SVG 檔案上限（位元組）。
const MAX_SVG_BYTES: u64 = 4 * 1024 * 1024;

/// 隨主題帶的單個字型檔案上限（位元組）：裁過字的中文字型在 10 MB 以內，整個主題包上限 32 MB。
const MAX_FONT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct Assets {
    /// 主題裡寫的路徑 → 解碼好的預乘點陣圖。
    images: HashMap<String, Arc<Pixmap>>,

    /// 主題裡寫的路徑 → 解析好的 SVG。
    svgs: HashMap<String, Arc<SvgImage>>,

    /// 隨主題帶的字型檔案。
    fonts: Vec<PathBuf>,
}

impl Assets {
    /// 讀 `file` 裡用到的全部圖片與字型；`dir` 是主題目錄。`inherited` 是所繼承內建主題編程序序的圖片：
    /// 主題目錄裡沒有的圖片到那裡找，`extends` 櫻花這類帶圖的內建主題、只改顏色字號時不用把圖拷一份。
    pub(crate) fn load(file: &ThemeFile, dir: &Path, inherited: &[(&str, &'static [u8])]) -> Self {
        let mut assets = Self::decode(file, |path, limit| match read_file(dir, path, limit) {
            Err(MISSING) => find_embedded(inherited, path),
            read => read,
        });
        for font in &file.fonts {
            match font_path(dir, &font.file) {
                Ok(path) if !assets.fonts.contains(&path) => assets.fonts.push(path),
                Ok(_) => {}
                Err(problem) => {
                    tracing::warn!(
                        id = file.meta.id,
                        path = font.file,
                        "主题字体{problem}，不用"
                    );
                }
            }
        }
        assets
    }

    /// 內建主題：圖片從編程序序的檔案表（路徑 → 內容）裡取。
    pub(crate) fn embedded(file: &ThemeFile, files: &[(&str, &'static [u8])]) -> Self {
        Self::decode(file, |path, _| find_embedded(files, path))
    }

    /// 解碼節點樹裡用到的全部圖片；`read` 按路徑給檔案內容，第二個引數是這種圖片的位元組上限。
    fn decode<'a>(
        file: &ThemeFile,
        read: impl Fn(&str, u64) -> Result<Cow<'a, [u8]>, &'static str>,
    ) -> Self {
        let mut images = HashMap::new();
        let mut svgs = HashMap::new();
        for path in image_paths(file) {
            if images.contains_key(&path) || svgs.contains_key(&path) {
                continue;
            }
            let loaded = if is_svg(&path) {
                read(&path, MAX_SVG_BYTES)
                    .and_then(|data| decode_svg(&data))
                    .map(|svg| {
                        svgs.insert(path.clone(), Arc::new(svg));
                    })
            } else {
                read(&path, u64::MAX)
                    .and_then(|data| decode_png(&data))
                    .map(|image| {
                        images.insert(path.clone(), Arc::new(image));
                    })
            };
            if let Err(problem) = loaded {
                tracing::warn!(id = file.meta.id, path, "主题图片{problem}，不画");
            }
        }
        Self {
            images,
            svgs,
            fonts: Vec::new(),
        }
    }

    pub(crate) fn image(&self, path: &str) -> Option<Arc<Pixmap>> {
        self.images.get(path).cloned()
    }

    pub(crate) fn svg(&self, path: &str) -> Option<Arc<SvgImage>> {
        self.svgs.get(path).cloned()
    }

    pub(crate) fn fonts(&self) -> &[PathBuf] {
        &self.fonts
    }
}

/// 主題裡寫的相對路徑 → 主題目錄裡的路徑；絕對路徑與 `..` 拒掉。
fn inside(dir: &Path, path: &str) -> Result<PathBuf, &'static str> {
    let relative = Path::new(path);
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err("路径不在主题目录里");
    }
    Ok(dir.join(relative))
}

fn font_path(dir: &Path, path: &str) -> Result<PathBuf, &'static str> {
    let path = inside(dir, path)?;
    let size = std::fs::metadata(&path).map_err(|_| "不存在")?.len();
    if size > MAX_FONT_BYTES {
        return Err("太大");
    }
    Ok(path)
}

fn is_svg(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
}

/// 檔案不存在時的問題描述（目錄裡沒有就去繼承的內建主題裡找）。
const MISSING: &str = "不存在";

fn find_embedded(
    files: &[(&str, &'static [u8])],
    path: &str,
) -> Result<Cow<'static, [u8]>, &'static str> {
    files
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, data)| Cow::Borrowed(*data))
        .ok_or(MISSING)
}

/// 主題目錄裡的檔案，超過 `limit` 位元組的不讀。
fn read_file(dir: &Path, path: &str, limit: u64) -> Result<Cow<'static, [u8]>, &'static str> {
    let path = inside(dir, path)?;
    if std::fs::metadata(&path).map_err(|_| MISSING)?.len() > limit {
        return Err("太大");
    }
    std::fs::read(&path).map(Cow::Owned).map_err(|_| "读不进来")
}

fn decode_svg(data: &[u8]) -> Result<SvgImage, &'static str> {
    let svg = SvgImage::parse(data).map_err(|_| "不是能解析的 SVG")?;
    let (width, height) = svg.size();
    if width > MAX_SIDE as f32 || height > MAX_SIDE as f32 {
        return Err("太大");
    }
    Ok(svg)
}

fn decode_png(data: &[u8]) -> Result<Pixmap, &'static str> {
    let image = Pixmap::decode_png(data).map_err(|_| "读不进来（只认 PNG）")?;
    if image.width() > MAX_SIDE || image.height() > MAX_SIDE {
        return Err("太大");
    }
    Ok(image)
}

/// 主題裡寫到的全部圖片路徑（可能重複）。
pub(super) fn image_paths(file: &ThemeFile) -> Vec<String> {
    let mut paths = Vec::new();
    for component in file.components.values() {
        collect(component, &mut paths);
    }
    collect(&file.windows.vertical, &mut paths);
    collect(&file.windows.horizontal, &mut paths);
    if let Some(root) = &file.status.root {
        collect(root, &mut paths);
    }
    paths
}

/// 收集節點樹裡的圖片路徑。
fn collect(node: &NodeSpec, out: &mut Vec<String>) {
    if let NodeKind::Frame { fill, children, .. } = &node.kind {
        if let Some(FillSpec::Image { image, .. }) = fill {
            out.push(image.clone());
        }
        for child in children {
            collect(child, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Assets;
    use crate::theme::Theme;

    const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"/>"#;

    #[test]
    fn embedded_files_are_found_by_theme_path() {
        let theme = Theme::from_json(
            r#"{ "extends": "qingjian", "schema": 1, "meta": { "id": "e", "name": "e" },
                 "windows": { "vertical": { "fill": { "image": "images/a.svg" } },
                              "horizontal": { "fill": { "image": "images/missing.png" } } } }"#,
            false,
        )
        .unwrap();
        let assets = Assets::embedded(theme.file(), &[("images/a.svg", SVG)]);
        assert!(assets.svg("images/a.svg").is_some());
        assert!(assets.image("images/missing.png").is_none());
        assert!(assets.fonts().is_empty());
    }
}
