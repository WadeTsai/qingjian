//! 候選視窗的點選區域：候選本體與它右側的各條譯詞各佔一塊，殼按滑鼠落點查是哪一塊。

mod region;
mod target;

pub use region::HitRegion;
pub use target::HitTarget;
