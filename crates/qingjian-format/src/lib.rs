//! `.qj` 資料容器：詞庫、語言模型這些常駐資料的二進位制檔案格式，開啟就是 mmap，不反序列化。
//!
//! 檔案 = [`Header`]（魔數、格式版本、資料種類、分節數）+ 分節表（4 位元組標籤 + 偏移 + 長度）+ 各分節正文，
//! 分節正文按 8 位元組對齊。第一節固定是 `META`：TOML 文本的 [`Metadata`]（名稱、許可證、署名、來源、條數），
//! 其餘分節由各資料種類自己定義（詞庫的 arena 與索引、語言模型的 CSR 陣列），記憶體裡是什麼佈局檔案裡就是什麼佈局，
//! 所以載入只是對映檔案再校驗一遍頭與分節邊界。
//!
//! 數值一律小端、原生對齊；本 crate 在大端機器上拒絕編譯。字串分節開啟時校驗一次 UTF-8。
//! 資料檔案只整體替換（寫臨時檔案再改名），從不就地修改。

#[cfg(target_endian = "big")]
compile_error!("qingjian-format 的 .qj 文件是小端布局零拷贝读取，不支持大端机器");

mod error;
mod file;
pub mod hash;
mod layout;
mod view;

pub use error::FormatError;
pub use file::{Container, Writer};
pub use layout::{FORMAT_VERSION, Header, Kind, MAGIC, Metadata, SectionEntry};
pub use view::{Table, Text};

/// 後設資料分節的標籤，每個檔案的第一節。
pub const META_TAG: [u8; 4] = *b"META";

/// 分節正文的對齊：足夠讓 `u64` / `f64` 欄位的結構體零複製讀取。
pub const SECTION_ALIGN: usize = 8;

/// 副檔名。
pub const EXTENSION: &str = "qj";

/// 分節標籤的可讀形式（日誌與錯誤用）。
pub fn tag_str(tag: &[u8; 4]) -> String {
    tag.iter()
        .map(|&b| if b.is_ascii_graphic() { b as char } else { '?' })
        .collect()
}
