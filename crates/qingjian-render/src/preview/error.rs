//! 主題預覽圖的錯誤：整體失敗（[`PreviewError`]）與單張失敗（[`ShotError`]）。

use std::path::PathBuf;

use crate::{RenderError, ThemeError};

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("theme could not be loaded: {0}")]
    Theme(#[from] ThemeError),

    #[error("fonts could not be loaded: {0}")]
    Fonts(#[from] RenderError),

    #[error("cannot create {path}: {source}")]
    OutDir {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ShotError {
    #[error("render failed: {0}")]
    Render(#[from] RenderError),

    #[error("cannot write {path}: {message}")]
    Save { path: PathBuf, message: String },
}
