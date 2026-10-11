//! 分節正文的兩種檢視：定長結構體陣列 [`Table`] 與 UTF-8 文本 [`Text`]。
//! 自己解析出來的資料和對映檔案裡的資料都用它們裝，查詢程式碼只見 `&[T]` / `&str`。

mod table;
mod text;

pub use table::Table;
pub use text::Text;
