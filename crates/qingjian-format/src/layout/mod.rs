//! 檔案裡的固定佈局：檔案頭、分節表條目、資料種類編號、後設資料分節的內容。

mod header;
mod kind;
mod metadata;
mod section_entry;

pub use header::{FORMAT_VERSION, Header, MAGIC};
pub use kind::Kind;
pub use metadata::Metadata;
pub use section_entry::SectionEntry;
