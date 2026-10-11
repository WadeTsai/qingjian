use std::io;

use crate::tag_str;

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("not a .qj file (bad magic)")]
    BadMagic,

    #[error("unsupported .qj format version {0}")]
    UnsupportedVersion(u16),

    #[error("wrong data kind: expected {expected:?}, found {found}")]
    WrongKind {
        /// 呼叫方要開啟的種類。
        expected: crate::Kind,

        /// 檔案頭裡的種類編號。
        found: u16,
    },

    #[error("file is truncated or section table is out of bounds")]
    Truncated,

    #[error("missing section {}", tag_str(.0))]
    MissingSection([u8; 4]),

    #[error("malformed section {}: {reason}", tag_str(.tag))]
    Malformed {
        /// 出問題的分節。
        tag: [u8; 4],

        /// 原因。
        reason: &'static str,
    },

    #[error("metadata is not valid TOML: {0}")]
    MetadataParse(#[from] toml::de::Error),

    #[error("metadata cannot be serialized: {0}")]
    MetadataSerialize(#[from] toml::ser::Error),

    #[error("section {} is too large", tag_str(.0))]
    SectionTooLarge([u8; 4]),
}
