use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write config {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid config {path}: {source}")]
    Parse {
        path: PathBuf,

        /// 裝著不拆：`toml::de::Error` 一百多位元組，直接放進列舉會把 `ConfigError`
        /// （以及各殼聚合它的錯誤型別）撐過 clippy 的 128 位元組閾值。
        #[source]
        source: Box<toml::de::Error>,
    },

    #[error("cannot edit config {path} in place: {source}")]
    Edit {
        path: PathBuf,

        /// 同上。
        #[source]
        source: Box<toml_edit::TomlError>,
    },
}
