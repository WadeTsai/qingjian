//! 雙拼：兩鍵一音節，第一鍵聲母、第二鍵韻母，零聲母另有約定。
//!
//! 解碼只做一件事：把敲的鍵翻成全拼（音節之間用 `'` 連上，切分因此沒有歧義），
//! 之後的切分、查詞、整句、聯想全部複用全拼的那一套；殼與詞庫都不知道雙拼的存在。
//! 上屏消耗按「音節對應幾個鍵」換算回緩衝區（見 [`Decoded::keys_for`]）。
//!
//! 各方案的鍵位表按 Rime 的 `double_pinyin*.schema.yaml` 核對（搜狗方案來自 rime-ice 的整理），
//! 見 [`Scheme`] 的各表；每套方案對全部音節做往返測試。

mod decoded;
mod scheme;
mod table;
mod unit;

pub use decoded::Decoded;
pub use scheme::Scheme;
pub use unit::Unit;
