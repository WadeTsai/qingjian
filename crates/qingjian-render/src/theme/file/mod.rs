//! 主題檔案 `theme.json` 的結構，見 `docs/design/theme.md`。只管反序列化，解析引用、例項化在別處。

mod adaptive;
mod color;
mod font;
mod meta;
pub(crate) mod node;
mod status;
mod text;
mod windows;

use std::collections::HashMap;

use serde::Deserialize;

pub(crate) use adaptive::Adaptive;
pub(crate) use color::{ColorRef, ColorSpec, ColorValue};
pub(crate) use font::{BundledFont, FamilyList, FontRef, FontStyle};
pub(crate) use meta::{LockedAppearance, Meta};
pub(crate) use status::StatusSpec;
pub(crate) use text::TextSettings;
pub(crate) use windows::Windows;

use node::NodeSpec;

/// 當前認得的最高格式版本。
pub(crate) const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ThemeFile {
    /// 格式版本，只在不相容改動時加一。
    pub(crate) schema: u32,

    pub(crate) meta: Meta,

    /// 命名顏色，節點裡用 `@名字` 引用。
    #[serde(default)]
    pub(crate) variables: HashMap<String, Adaptive<ColorValue>>,

    pub(crate) text: TextSettings,

    /// 隨主題帶的字型檔案。
    #[serde(default)]
    pub(crate) fonts: Vec<BundledFont>,

    /// 可複用的節點（元件），`use` / `repeat` 按名字引用。
    #[serde(default)]
    pub(crate) components: HashMap<String, NodeSpec>,

    pub(crate) windows: Windows,

    pub(crate) status: StatusSpec,
}
