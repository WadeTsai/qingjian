//! 使用者指定的介面字型：字族名加它的字型檔案。檔案由殼按平臺的字型登記查出來（macOS 走 CoreText），渲染器只負責載入。

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiFont {
    /// 字族名，如 `LXGW WenKai`。
    pub family: String,

    /// 這個字族的字型檔案；空表示殼沒找到。
    pub files: Vec<PathBuf>,
}
