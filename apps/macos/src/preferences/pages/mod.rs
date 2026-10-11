//! 偏好設定的各頁：一頁一個檔案，各自建控制元件（`build`）、按配置重新整理（`sync`）。
//! 視窗本體（`window.rs`）只負責把頁裝進標籤檢視和底部狀態行。

mod about;
mod advanced;
mod candidates;
mod cloud;
mod dictionaries;
mod fuzzy;
mod general;
mod phrases;
mod shortcuts;
mod usage;

pub(super) use about::AboutPage;
pub(super) use about::build as build_about;
pub use about::{REPOSITORY_URL, UpdateStatus, WEBSITE_URL};
pub(super) use advanced::AdvancedPage;
pub(super) use candidates::CandidatesPage;
pub(super) use cloud::CloudPage;
pub(super) use dictionaries::DictionariesPage;
pub(super) use fuzzy::FuzzyPage;
pub(super) use general::GeneralPage;
pub(super) use phrases::PhrasesPage;
pub(super) use shortcuts::ShortcutsPage;
pub(super) use usage::UsagePage;
