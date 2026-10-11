//! 讀與寫：[`Container`] 對映並校驗一個 `.qj` 檔案，[`Writer`] 攢分節寫出去。

mod container;
mod writer;

pub use container::Container;
pub use writer::Writer;
