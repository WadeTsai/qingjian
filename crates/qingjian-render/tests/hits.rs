//! 候選視窗的點選區域：每個候選一塊、每條譯詞一塊，點在哪塊上就是哪個目標；各內建主題、橫豎排都要有。

mod scenes;

use qingjian_render::{FontLibrary, HitRegion, HitTarget, Layout, Renderer, Theme};

fn region(hits: &[HitRegion], target: HitTarget) -> HitRegion {
    *hits
        .iter()
        .find(|region| region.target == target)
        .unwrap_or_else(|| panic!("没有 {target:?} 的点击区域：{hits:?}"))
}

fn center(region: HitRegion, scale: f32) -> (f32, f32) {
    (
        (region.x + region.width / 2.0) / scale,
        (region.y + region.height / 2.0) / scale,
    )
}

#[test]
fn candidates_and_their_translations_are_clickable() {
    let Ok(library) = FontLibrary::system("zh-CN") else {
        eprintln!("没有系统字体，跳过");
        return;
    };
    let mut renderer = Renderer::new(library);
    let frame = scenes::nihao();
    let themes = [
        ("qingjian", Theme::light()),
        ("system-blue", Theme::builtin("system-blue", false).unwrap()),
        ("wechat", Theme::builtin("wechat", false).unwrap()),
        ("sakura", Theme::builtin("sakura", false).unwrap()),
    ];
    for (name, theme) in &themes {
        for layout in [Layout::Vertical, Layout::Horizontal] {
            let rendered = renderer.render(&frame, layout, theme, 2.0).unwrap();
            let hits = &rendered.hits;
            let what = format!("{name} {layout:?}");
            for i in 0..frame.rows.len() {
                region(hits, HitTarget::Candidate(i));
            }
            // 「你好」有兩條譯詞 hello · hi；主題只畫第一條時第二條不可點
            let first = region(
                hits,
                HitTarget::Translation {
                    candidate: 0,
                    sense: 0,
                },
            );
            let (x, y) = center(first, rendered.scale);
            assert_eq!(rendered.hit_at(x, y), Some(first.target), "{what}");
            let second = hits.iter().find(|region| {
                region.target
                    == HitTarget::Translation {
                        candidate: 0,
                        sense: 1,
                    }
            });
            if let Some(&second) = second {
                assert!(
                    first.x + first.width <= second.x,
                    "{what}: {first:?} {second:?}"
                );
                let (x, y) = center(second, rendered.scale);
                assert_eq!(rendered.hit_at(x, y), Some(second.target), "{what}");
            }
            // 候選本體：從候選區域左邊往裡一點，落在序號 / 詞上
            let word = region(hits, HitTarget::Candidate(1));
            let (x, y) = (
                (word.x + 2.0) / rendered.scale,
                (word.y + word.height / 2.0) / rendered.scale,
            );
            assert_eq!(
                rendered.hit_at(x, y),
                Some(HitTarget::Candidate(1)),
                "{what}"
            );
            assert_eq!(rendered.hit_at(-1.0, -1.0), None, "{what}");
        }
    }
}
