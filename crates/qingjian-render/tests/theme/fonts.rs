//! 主題字型：樣式寫的系統字族由殼查檔案交給渲染器載入，隨主題帶的字型從主題目錄載入；回退鏈挑第一個裝了的。
//! 用系統自帶的 Georgia 當樣例，系統裡沒有時跳過。

use std::path::{Path, PathBuf};

use qingjian_render::{FontLibrary, Renderer, Theme};

/// 這個平臺上 Georgia 的檔案。
fn georgia() -> Option<PathBuf> {
    [
        "/System/Library/Fonts/Supplemental/Georgia.ttf",
        r"C:\Windows\Fonts\georgia.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.is_file())
}

fn renderer() -> Option<Renderer> {
    FontLibrary::system("zh-CN").ok().map(Renderer::new)
}

fn theme_json(candidate_family: &str, fonts: &str) -> String {
    format!(
        r#"{{ "extends": "qingjian", "schema": 1, "meta": {{ "id": "fonts", "name": "fonts" }}, "fonts": {fonts},
             "text": {{ "styles": {{ "candidate": {{ "size": 16, "line_height": 19, "family": {candidate_family} }} }} }} }}"#
    )
}

#[test]
fn system_family_is_loaded_through_the_shell_lookup() {
    let (Some(file), Some(mut renderer)) = (georgia(), renderer()) else {
        eprintln!("没有 Georgia 或系统字体，跳过");
        return;
    };
    let theme = Theme::from_json(
        &theme_json(r#"["No Such Font", "Georgia", "system"]"#, "[]"),
        false,
    )
    .unwrap();
    assert_eq!(theme.font_families(), ["No Such Font", "Georgia"]);
    let before = renderer.trace_families("Abc", &theme);
    assert!(!before.contains(&"Georgia".to_owned()), "{before:?}");

    let mut asked = Vec::new();
    let lookup = |family: &str| {
        if family == "Georgia" {
            vec![file.clone()]
        } else {
            Vec::new()
        }
    };
    renderer.load_theme_fonts(&theme, |family| {
        asked.push(family.to_owned());
        lookup(family)
    });
    assert_eq!(asked, ["No Such Font", "Georgia"]);
    assert_eq!(renderer.trace_families("Abc", &theme), ["Georgia"]);
}

#[test]
fn missing_families_fall_back_to_the_ui_font() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let plain = Theme::light();
    let theme = Theme::from_json(&theme_json(r#""No Such Font""#, "[]"), false).unwrap();
    renderer.load_theme_fonts(&theme, |_| Vec::new());
    assert_eq!(
        renderer.trace_families("Abc", &theme),
        renderer.trace_families("Abc", &plain)
    );
}

#[test]
fn bundled_font_loads_from_the_theme_directory() {
    let (Some(file), Some(mut renderer)) = (georgia(), renderer()) else {
        eprintln!("没有 Georgia 或系统字体，跳过");
        return;
    };
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("theme-fonts");
    std::fs::create_dir_all(dir.join("fonts")).unwrap();
    std::fs::copy(&file, dir.join("fonts/bundled.ttf")).unwrap();
    let json = theme_json(
        r#""Georgia""#,
        r#"[{ "file": "fonts/bundled.ttf" }, { "file": "../outside.ttf" }, { "file": "fonts/missing.ttf" }]"#,
    );
    std::fs::write(dir.join("theme.json"), json).unwrap();

    let theme = Theme::from_dir(&dir, false).unwrap();
    assert_eq!(theme.font_files(), [dir.join("fonts/bundled.ttf")]);
    // 隨包字型先載入，字型庫裡有了 Georgia 就不再向殼查
    renderer.load_theme_fonts(&theme, |family| panic!("不该查 {family}"));
    assert_eq!(renderer.trace_families("Abc", &theme), ["Georgia"]);
    assert!(Path::new(&theme.font_files()[0]).starts_with(&dir));
}
