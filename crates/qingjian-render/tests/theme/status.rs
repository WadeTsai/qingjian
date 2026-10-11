//! 狀態條的節點樹畫法：格子按 `repeat` 繫結 `cells` 展開，點選邊界按各格盒子算；輸入狀態能用 `when` 分支。

use std::path::PathBuf;

use qingjian_render::{FontLibrary, Mode, Renderer, StatusCell, Theme};

/// 左邊按中英各畫一個同尺寸不同色的方塊（不用字：Linux CI 沒有中文字型，兩個字都畫成同一個缺字框），
/// 格子之間各夾一個 10 點寬的裝飾，格子 4 點內邊距。
const THEME: &str = r##"{
    "extends": "qingjian", "schema": 1, "meta": { "id": "status-tree", "name": "status-tree" },
    "components": {
        "cell": { "type": "frame", "padding": 4, "children": [
            { "type": "text", "bind": "text", "font": "candidate", "color": "@text" },
            { "type": "icon", "icon": "gear", "when": "gear", "size": 12, "color": "@gloss" }
        ] },
        "status-item": { "type": "frame", "children": [
            { "type": "frame", "width": 10, "height": 10, "when": "!first", "fill": { "image": "images/dot.svg" } },
            { "type": "use", "component": "cell" }
        ] }
    },
    "status": { "root": { "type": "frame", "fill": "@surface", "children": [
        { "type": "frame", "width": 16, "height": 16, "when": "mode.english", "fill": "#0000ff" },
        { "type": "frame", "width": 16, "height": 16, "when": "!mode.english", "fill": "#00ff00" },
        { "type": "repeat", "bind": "cells", "component": "status-item" }
    ] } }
}"##;

const DOT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><circle cx="5" cy="5" r="5" fill="#ff0000"/></svg>"##;

fn theme() -> Theme {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("status-tree");
    std::fs::create_dir_all(dir.join("images")).unwrap();
    std::fs::write(dir.join("images/dot.svg"), DOT).unwrap();
    std::fs::write(dir.join("theme.json"), THEME).unwrap();
    Theme::from_dir(&dir, false).unwrap()
}

#[test]
fn status_tree_cells_and_mode() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let mut renderer = Renderer::new(library);
    let theme = theme();
    let cells = [
        StatusCell::text("中 · 小鹤", true),
        StatusCell::text(",.", false),
        StatusCell::Gear,
    ];
    let chinese = renderer
        .render_status(&cells, &Mode::default(), &theme, 2.0)
        .unwrap();
    let edges = &chinese.cell_edges;
    assert_eq!(edges.len(), 3);
    assert!(edges.windows(2).all(|pair| pair[0] < pair[1]), "{edges:?}");
    assert_eq!(
        *edges.last().unwrap(),
        chinese.rendered.content_width as f32
    );
    // 第一格前面是中文模式的方塊：第一格的右邊界比它自己的寬度靠右
    assert!(edges[0] > 0.0);

    // 第二格前的紅點裝飾畫出來了：在第一格右邊界之後 10 點（20 畫素）以內找紅色
    let pixmap = &chinese.rendered.pixmap;
    let (cx, cy) = (chinese.rendered.content_x, chinese.rendered.content_y);
    let y = cy + 10;
    let red = (edges[0] as u32..edges[0] as u32 + 20).any(|x| {
        let pixel = pixmap.pixel(cx + x, y).unwrap().demultiply();
        pixel.red() > 200 && pixel.green() < 60
    });
    assert!(red, "格子之间的装饰没画出来");

    // 英文模式左邊換成另一色的方塊：同寬，格子邊界不變
    let english = renderer
        .render_status(
            &cells,
            &Mode {
                english: true,
                ..Mode::default()
            },
            &theme,
            2.0,
        )
        .unwrap();
    assert_eq!(english.cell_edges, chinese.cell_edges);
    assert_ne!(
        english.rendered.pixmap.data(),
        chinese.rendered.pixmap.data()
    );
}
