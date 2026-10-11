//! 標記文本：preedit 裡按分段畫的拼音行，每段帶種類（正常 / 糾錯 / 未解析等）。

mod aux_segment;
mod kind;
mod segment;

pub use aux_segment::AuxSegment;
pub use kind::MarkedKind;
pub use segment::MarkedSegment;
