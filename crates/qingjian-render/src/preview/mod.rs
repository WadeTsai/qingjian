//! 主題預覽圖：用樣例幀把一個主題目錄畫成投稿用的截圖，寫進它的 `preview/`。
//! 兩個殼的 `--theme-preview <主題目錄>` 調它，主題倉庫的 `tools/preview.py` 再調殼；規則見主題倉庫 README「提交主題」。

mod error;
mod shot;

use std::path::{Path, PathBuf};

pub use error::{PreviewError, ShotError};
pub use shot::Shot;

use crate::sample::{nihao, nihao_with_sentence};
use crate::{FontLibrary, Frame, Layout, Renderer, Theme};

/// 2 倍圖：與 Retina 截圖一樣清楚，單張在主題倉庫的 500 KB 上限以內。
const SCALE: f32 = 2.0;

/// 命令列 `--theme-preview` 的命令名。
pub const FLAG: &str = "--theme-preview";

/// 殼的命令列入口：把 `theme_dir` 的預覽圖寫進 `theme_dir/preview/`，逐張列印結果，返回程序退出碼。
/// 至少寫出一張算成功（缺的那張單獨列出來，不讓整個命令失敗）；主題讀不進來或一張都沒寫出是 1。
pub fn run(theme_dir: &Path, family_files: impl FnMut(&str) -> Vec<PathBuf>) -> i32 {
    let shots = match write_previews(theme_dir, &theme_dir.join("preview"), family_files) {
        Ok(shots) => shots,
        Err(error) => {
            eprintln!("无法生成预览图：{error}");
            return 1;
        }
    };
    let mut missing = Vec::new();
    for shot in &shots {
        match &shot.result {
            Ok(path) => println!("已写出 {}", path.display()),
            Err(error) => {
                eprintln!("{} 生成失败：{error}", shot.file);
                missing.push(shot.file);
            }
        }
    }
    if missing.len() == shots.len() {
        return 1;
    }
    if !missing.is_empty() {
        eprintln!("缺 {}，其余已写出", missing.join("、"));
    }
    0
}

/// 畫 `theme_dir` 的豎排、橫排截圖（主題沒鎖定外觀時再加一張深色豎排），寫到 `out_dir`，同名檔案覆蓋。
/// 某一張畫壞了記在它的 [`Shot`] 裡、其餘照畫；主題讀不進來或字型庫建不起來才整體失敗。
/// `family_files` 是殼按字族名找系統字型檔案的函式，與候選窗載入主題字型時同一個。
pub fn write_previews(
    theme_dir: &Path,
    out_dir: &Path,
    family_files: impl FnMut(&str) -> Vec<PathBuf>,
) -> Result<Vec<Shot>, PreviewError> {
    let theme = Theme::from_dir(theme_dir, false)?;
    let mut renderer = Renderer::new(FontLibrary::system("zh-CN")?);
    renderer.load_theme_fonts(&theme, family_files);
    std::fs::create_dir_all(out_dir).map_err(|source| PreviewError::OutDir {
        path: out_dir.to_owned(),
        source,
    })?;
    let mut shots = vec![
        ("vertical.png", nihao(), Layout::Vertical, false),
        (
            "horizontal.png",
            nihao_with_sentence(),
            Layout::Horizontal,
            false,
        ),
    ];
    if theme.locked_dark().is_none() {
        shots.push(("vertical-dark.png", nihao(), Layout::Vertical, true));
    }
    Ok(shots
        .into_iter()
        .map(|(file, frame, layout, dark)| Shot {
            file,
            result: draw(
                &mut renderer,
                &theme.with_dark(dark),
                &frame,
                layout,
                &out_dir.join(file),
            ),
        })
        .collect())
}

fn draw(
    renderer: &mut Renderer,
    theme: &Theme,
    frame: &Frame,
    layout: Layout,
    path: &Path,
) -> Result<PathBuf, ShotError> {
    // 每張獨立，不和上一張配對播過渡
    renderer.forget();
    let rendered = renderer.render(frame, layout, theme, SCALE)?;
    rendered
        .pixmap
        .save_png(path)
        .map_err(|source| ShotError::Save {
            path: path.to_owned(),
            message: source.to_string(),
        })?;
    Ok(path.to_owned())
}
