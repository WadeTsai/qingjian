//! 離線預覽：`cargo run --release -p qingjian-render --example preview -- --out target/render-preview`
//! 把樣例幀按淺 / 深色、豎 / 橫排畫成 PNG，與各平臺原生候選窗截圖並排比；`--measure` 只量幾段文字的寬度與原生對數；
//! 末尾列出驗收行每個字形落到了哪家字型。不是日常工具，改渲染器時拿來核對。
//! `--theme <主題目錄>` 畫使用者主題；主題樣式寫的系統字族在 `--font-dir`（可寫多個）裡按字族名找檔案，代替殼的系統字型登記。

use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::Parser;
use qingjian_render::{FontLibrary, Mode, Renderer, Theme};

#[path = "../tests/scenes/mod.rs"]
mod scenes;

#[derive(Parser)]
struct Args {
    /// PNG 輸出目錄。
    #[arg(long, default_value = "target/render-preview")]
    out: PathBuf,

    /// 點 → 畫素倍數（Retina 為 2）。
    #[arg(long, default_value_t = 2.0)]
    scale: f32,

    /// 中日字形回退用的 locale。
    #[arg(long, default_value = "zh-CN")]
    locale: String,

    /// 只量幾段文字的寬度（點），不出圖；與 AppKit 的 NSAttributedString.size() 對數。
    #[arg(long)]
    measure: bool,

    /// 畫這個主題目錄（含 theme.json），不寫畫預設內建主題。
    #[arg(long)]
    theme: Option<PathBuf>,

    /// 主題寫的系統字族到這些目錄裡找字型檔案。
    #[arg(long = "font-dir", default_values = default_font_dirs())]
    font_dirs: Vec<PathBuf>,
}

/// 各平臺放系統字型的目錄。
fn default_font_dirs() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec![
            "/System/Library/Fonts",
            "/System/Library/Fonts/Supplemental",
            "/Library/Fonts",
        ]
    } else if cfg!(target_os = "windows") {
        vec![r"C:\Windows\Fonts"]
    } else {
        vec!["/usr/share/fonts"]
    }
}

/// 在字型目錄裡按字族名找檔案（只看名字表，不載入）。
fn family_files(dirs: &[PathBuf], family: &str) -> Vec<PathBuf> {
    let mut db = cosmic_text::fontdb::Database::new();
    for dir in dirs {
        db.load_fonts_dir(dir);
    }
    let mut files = Vec::new();
    for face in db.faces() {
        let cosmic_text::fontdb::Source::File(path) = &face.source else {
            continue;
        };
        if face
            .families
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case(family))
            && !files.contains(path)
        {
            files.push(path.clone());
        }
    }
    files
}

fn load_theme(dir: &Path, dark: bool) -> Result<Theme, Box<dyn std::error::Error>> {
    Ok(Theme::from_dir(dir, dark)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "qingjian_render=debug".into()),
        )
        .init();
    let args = Args::parse();
    std::fs::create_dir_all(&args.out)?;

    let started = Instant::now();
    let library = FontLibrary::system(&args.locale)?;
    println!(
        "字体库：{:?}，界面字体 {}",
        started.elapsed(),
        library.ui_family()
    );
    println!("已加载字族：{}", library.families().join(" / "));
    let mut renderer = Renderer::new(library);
    if args.measure {
        for text in [
            "int. hello · int. hi",
            "hello",
            "ni'hao",
            "1/6",
            "你好",
            "phr. you change",
            "int. ",
            "·",
            " · ",
            "hi",
            "你好像",
            "開発する",
        ] {
            let widths: Vec<String> = [11.0, 12.0, 16.0]
                .into_iter()
                .map(|size| format!("{size}pt={:.2}", renderer.measure_points(text, size)))
                .collect();
            println!("{text:<24} {}", widths.join("  "));
        }
        return Ok(());
    }
    let themes = match &args.theme {
        Some(dir) => {
            let themes = [
                ("light", load_theme(dir, false)?),
                ("dark", load_theme(dir, true)?),
            ];
            renderer.load_theme_fonts(&themes[0].1, |family| family_files(&args.font_dirs, family));
            themes
        }
        None => [("light", Theme::light()), ("dark", Theme::dark())],
    };
    let samples = scenes::candidate_scenes();
    for (theme_name, theme) in &themes {
        for (scene, frame, layout) in &samples {
            let started = Instant::now();
            // 每張樣例獨立：不和上一張配對播過渡
            renderer.forget();
            let rendered = renderer.render(frame, *layout, theme, args.scale)?;
            let elapsed = started.elapsed();
            let path = args.out.join(format!("{scene}-{theme_name}.png"));
            rendered.pixmap.save_png(&path)?;
            let (w, h) = rendered.content_size_points();
            println!(
                "{:<28} {:>4.0}×{:<4.0}pt  {:>8.2?}  {}",
                format!("{scene}-{theme_name}"),
                w,
                h,
                elapsed,
                path.display()
            );
        }
    }

    // Windows 的懸浮狀態條：三格
    let cells = scenes::status_cells();
    for (theme_name, theme) in &themes {
        let status = renderer.render_status(&cells, &Mode::default(), theme, args.scale)?;
        let path = args.out.join(format!("status-{theme_name}.png"));
        status.rendered.pixmap.save_png(&path)?;
        let (w, h) = status.rendered.content_size_points();
        println!(
            "{:<28} {:>4.0}×{:<4.0}pt  格边界 {:?}  {}",
            format!("status-{theme_name}"),
            w,
            h,
            status.cell_edges,
            path.display()
        );
    }

    for probe in [
        "青简 hello 🙂 日本語 骨直曜",
        "開発(かいはつ)する",
        "int. hello · int. hi",
    ] {
        println!(
            "「{probe}」各字形字体：{}",
            renderer.trace_families(probe, &themes[0].1).join(" → ")
        );
    }
    Ok(())
}
