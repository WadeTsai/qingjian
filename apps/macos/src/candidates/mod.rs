//! 候選視窗：自繪 NSPanel 與檢視、一幀的資料（preedit / 行 / 頁尾）、主題。

mod animation;
mod bitmap;
mod bounds;
mod cloud_icon;
mod frame;
mod preedit;
mod row;
mod theme;
mod view;
mod window;

pub(crate) use bitmap::{available_families, family_files};
pub use frame::Frame;
pub use preedit::Preedit;
pub use row::Row;
pub use window::CandidateWindow;
