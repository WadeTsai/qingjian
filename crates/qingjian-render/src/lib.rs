//! 自繪渲染器：把候選窗的一幀（拼音行、候選行、頁尾）按主題畫成一張點陣圖，各平臺殼只負責把點陣圖貼到視窗上。
//!
//! 柵格用 tiny-skia（純 CPU），文字用 cosmic-text（fontdb 選字型 + 整形 + swash 柵格，帶回退鏈與彩色 emoji）。
//! 字型不掃系統目錄，按平臺清單隻載入介面字型、中文、日文、emoji 幾個檔案（[`FontLibrary`]）。
//! 所有尺寸以「點」為單位寫在 [`Theme`] 裡，[`Renderer`] 按縮放倍數換成畫素；輸出是預乘 alpha 的 RGBA 點陣圖。
//!
//! 設計與驗收見 `docs/design/rendering.md`。

mod animation;
mod canvas;
mod cloud;
mod color;
mod error;
mod fonts;
mod frame;
mod gear;
mod layout;
pub mod preview;
mod renderer;
pub mod sample;
mod scene;
mod shadow;
mod text;
mod theme;

pub use color::Color;
pub use error::RenderError;
/// Windows 的字型登記：按字族名找檔案、列字族名（設定頁用）。
#[cfg(target_os = "windows")]
pub use fonts::directwrite as system_fonts;
pub use fonts::{FontLibrary, UiFont};
pub use frame::{Frame, Mode, Preedit, PreeditSegment, PreeditStyle, Row, Tone};
pub use layout::Layout;
pub use renderer::{HitRegion, HitTarget, Rendered, RenderedStatus, Renderer, StatusCell};
pub use theme::{FontSpec, TextSizes, Theme, ThemeError, ThemeLibrary};

/// 讓 `tiny_skia::Pixmap` 的使用方不用再單獨依賴 tiny-skia。
pub use tiny_skia::Pixmap;
