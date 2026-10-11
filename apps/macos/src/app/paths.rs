//! 資料檔案位置：只讀資料在 `.app/Contents/Resources/`，使用者資料在 `~/Library/Application Support/Qingjian/`。

use std::path::PathBuf;

use objc2_foundation::NSBundle;

use crate::error::HostError;

/// 主 bundle 的 Resources 目錄。
pub fn resources_dir() -> Result<PathBuf, HostError> {
    NSBundle::mainBundle()
        .resourcePath()
        .map(|p| PathBuf::from(p.to_string()))
        .ok_or(HostError::NoResources)
}

/// 某個資原始檔的完整路徑，不存在時報錯而不是等到讀取時才炸。
pub fn resource(name: &str) -> Result<PathBuf, HostError> {
    let path = resources_dir()?.join(name);
    if path.is_file() {
        Ok(path)
    } else {
        Err(HostError::MissingResource(path))
    }
}

/// 隨包的領域詞庫目錄：`.app/Contents/Resources/dicts/`；包裡沒有就是 `None`。
pub fn bundled_dicts_dir() -> Option<PathBuf> {
    let dir = resources_dir().ok()?.join("dicts");
    dir.is_dir().then_some(dir)
}

/// 形碼碼錶（五筆）：使用者目錄 `wubi/wubi86.tsv` 裡有就用它（自己換的表），
/// 否則用包裡的 `Resources/wubi/wubi86.tsv`；都沒有是 `None`。
pub fn code_table_path() -> Option<PathBuf> {
    let user = user_data_dir()?.join("wubi/wubi86.tsv");
    if user.is_file() {
        return Some(user);
    }
    let bundled = resources_dir().ok()?.join("wubi/wubi86.tsv");
    bundled.is_file().then_some(bundled)
}

/// 配置檔案：`~/Library/Application Support/Qingjian/config.toml`。
pub fn config_file() -> Option<PathBuf> {
    user_data_dir().map(|dir| dir.join("config.toml"))
}

/// 使用者資料目錄，不存在則建立。
pub fn user_data_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/Qingjian");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 使用者主題目錄：`~/Library/Application Support/Qingjian/themes/`，一個主題一個子目錄（`<id>/theme.json`），不存在則建立。
pub fn themes_dir() -> Option<PathBuf> {
    let dir = user_data_dir()?.join("themes");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 附加詞庫目錄：`~/Library/Application Support/Qingjian/dicts/`，不存在則建立。
pub fn dicts_dir() -> Option<PathBuf> {
    let dir = user_data_dir()?.join("dicts");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 含章·知微（`.qjm` 單檔案，或開發時的三件套目錄）：
/// 使用者目錄 `models/hanzhang-zhiwei/` 優先，相容舊 `model/`；否則用包裡的同名目錄。
pub fn model_path() -> Option<PathBuf> {
    let user = user_data_dir()?;
    for dir in ["models/hanzhang-zhiwei", "model"] {
        if let Some(found) = qingjian_neural::find_model(&user.join(dir)) {
            return Some(found);
        }
    }
    qingjian_neural::find_model(&resources_dir().ok()?.join("models/hanzhang-zhiwei"))
}

/// 含章·通變（P2C，帶噪拼音 → 漢字）：使用者新目錄優先，相容舊 `model-p2c/`。
pub fn p2c_model_path() -> Option<PathBuf> {
    let user = user_data_dir()?;
    for dir in ["models/hanzhang-tongbian", "model-p2c"] {
        if let Some(found) = qingjian_neural::find_model(&user.join(dir)) {
            return Some(found);
        }
    }
    qingjian_neural::find_model(&resources_dir().ok()?.join("models/hanzhang-tongbian"))
}
