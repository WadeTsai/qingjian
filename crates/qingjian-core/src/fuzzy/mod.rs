//! 模糊音：把使用者敲的音節擴充套件成若干等價寫法（`zi` 也查 `zhi`，`lan` 也查 `nan`，`fen` 也查 `feng`），
//! 詞庫按「每個位置多種寫法」一次查出，排序時模糊命中的詞頻減半（見 `ranking`）。
//!
//! 聲母規則對完整音節和字首 / 簡拼都生效（字首 `zh` 開了 z/zh 就換成 `z`，它本來就覆蓋 `zh`）；
//! 韻母規則只對完整音節生效：末尾未打完的音節按字首查，`fen` 自然能出 `feng`。

mod expanded;
mod rules;

pub use expanded::Expanded;
pub use rules::FuzzyRules;
