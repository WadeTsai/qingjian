//! 主題裡的 SVG 圖片：視窗底用九宮格、超出主題目錄的路徑不讀。走完整的渲染器，按畫素核對。

use std::path::{Path, PathBuf};

use qingjian_render::{FontLibrary, Layout, Rendered, Renderer, Theme};

use crate::scenes;

/// 四邊各 4 個單位紅邊、中間藍色的 20×20 SVG。
const FRAME: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
    <rect width="20" height="20" fill="#ff0000"/>
    <rect x="4" y="4" width="12" height="12" fill="#0000ff"/>
</svg>"##;

fn theme_dir(name: &str, image: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(dir.join("images")).unwrap();
    std::fs::write(dir.join("images/frame.svg"), FRAME).unwrap();
    let json = format!(
        r#"{{ "extends": "qingjian", "schema": 1, "meta": {{ "id": "{name}", "name": "{name}" }},
             "windows": {{ "vertical": {{ "radius": 0, "effects": [],
                 "fill": {{ "image": "{image}", "slice": [4, 4, 4, 4] }} }} }} }}"#
    );
    std::fs::write(dir.join("theme.json"), json).unwrap();
    dir
}

fn render(dir: &Path) -> Option<Rendered> {
    let mut renderer = Renderer::new(FontLibrary::system("zh-CN").ok()?);
    let theme = Theme::from_dir(dir, false).unwrap();
    Some(
        renderer
            .render(&scenes::nihao(), Layout::Vertical, &theme, 2.0)
            .unwrap(),
    )
}

/// 內容區裡 `(x, y)` 畫素（相對內容區左上角）的 RGBA。
fn pixel(rendered: &Rendered, x: u32, y: u32) -> [u8; 4] {
    let pixel = rendered
        .pixmap
        .pixel(rendered.content_x + x, rendered.content_y + y)
        .unwrap()
        .demultiply();
    [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
}

#[test]
fn svg_nine_slice_keeps_corners_at_scale() {
    let Some(rendered) = render(&theme_dir("svg-frame", "images/frame.svg")) else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let width = rendered.content_width;
    // 2 倍屏：4 個單位的紅邊是 8 個畫素，寬度不隨視窗拉伸
    assert_eq!(pixel(&rendered, 1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(&rendered, width / 2, 7), [255, 0, 0, 255]);
    assert_eq!(pixel(&rendered, width / 2, 8)[2], 255);
    assert_eq!(pixel(&rendered, width - 2, 1), [255, 0, 0, 255]);
}

#[test]
fn svg_outside_theme_directory_is_not_drawn() {
    let Some(rendered) = render(&theme_dir("svg-outside", "../svg-frame/images/frame.svg")) else {
        return;
    };
    assert_eq!(pixel(&rendered, 1, 1)[3], 0);
}
