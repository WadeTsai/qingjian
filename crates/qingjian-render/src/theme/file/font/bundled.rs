//! 隨主題帶的字型檔案：`"fonts": [{ "file": "fonts/LXGWWenKai.ttf" }]`。
//! 字族名從字型檔案裡讀，樣式的 `family` 寫這個名字；只能帶許可允許再分發的字型。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BundledFont {
    /// 相對 `theme.json` 的路徑（TTF / OTF / TTC）。
    pub(crate) file: String,
}
