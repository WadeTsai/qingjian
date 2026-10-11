//! macOS 的字型檔案清單。系統字型 SF 與 PingFang 不在 `/Library/Fonts`，路徑是系統私有位置，版本升級可能挪，所以每個角色給幾個候選。

use std::path::PathBuf;

const SYSTEM_FONTS: &str = "/System/Library/Fonts";

/// 介面字型：SF Pro（檔名 SFNS，字族名 `.SF NS`）。
pub(super) fn ui_fonts() -> Vec<PathBuf> {
    vec![
        PathBuf::from(format!("{SYSTEM_FONTS}/SFNS.ttf")),
        PathBuf::from(format!("{SYSTEM_FONTS}/Helvetica.ttc")),
    ]
}

/// 介面字型的其餘字重：SF 是可變字型、蘋方是多字重集合，不用另加檔案。
pub(super) fn ui_weight_fonts() -> Vec<PathBuf> {
    Vec::new()
}

/// 按 locale 要的漢字字型，加日文作為日語譯文的回退。
pub(super) fn script_fonts(locale: &str) -> Vec<PathBuf> {
    let mut fonts = Vec::new();
    if !locale.starts_with("ja") {
        fonts.extend(pingfang_paths());
        // 沒找到 PingFang 時的簡體兜底
        fonts.push(PathBuf::from(format!(
            "{SYSTEM_FONTS}/Hiragino Sans GB.ttc"
        )));
        // 宋體是靜態 glyf：macOS 26+ 的蘋方換成 swash 讀不出的 hvgl 輪廓（fonts/mod.rs 會跳過），
        // 這時簡體回退落到宋體，候選不至於空白
        fonts.push(PathBuf::from(format!(
            "{SYSTEM_FONTS}/Supplemental/Songti.ttc"
        )));
    }
    fonts.push(PathBuf::from(format!(
        "{SYSTEM_FONTS}/ヒラギノ角ゴシック W4.ttc"
    )));
    fonts
}

pub(super) fn emoji_fonts() -> Vec<PathBuf> {
    vec![PathBuf::from(format!(
        "{SYSTEM_FONTS}/Apple Color Emoji.ttc"
    ))]
}

/// PingFang.ttc 在系統資產目錄裡，目錄名帶雜湊，要掃一層找出來；找不到退回 FontServices 裡的 UI 版。
fn pingfang_paths() -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(assets) = std::fs::read_dir("/System/Library/AssetsV2") {
        for bucket in assets.flatten() {
            if !bucket
                .file_name()
                .to_string_lossy()
                .starts_with("com_apple_MobileAsset_Font")
            {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(bucket.path()) else {
                continue;
            };
            for asset in entries.flatten() {
                let candidate = asset.path().join("AssetData/PingFang.ttc");
                if candidate.is_file() {
                    found.push(candidate);
                }
            }
        }
    }
    found.push(PathBuf::from(
        "/System/Library/PrivateFrameworks/FontServices.framework/Versions/A/Resources/Reserved/PingFangUI.ttc",
    ));
    found
}
