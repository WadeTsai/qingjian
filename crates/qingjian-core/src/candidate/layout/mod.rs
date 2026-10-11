//! 候選視窗的排布：本地候選按頁排，雲聯想的詞到了以後只補進第一頁末尾的幾格。
//!
//! 雲端詞晚到幾百毫秒到一秒。以前它插在本地候選第二位，使用者看著 2 號按下去的一瞬間 2 號被雲端詞頂掉，選錯。
//! 現在第一頁前 `page_size−slots` 格永遠是本地候選，雲端詞到了填進最後 `slots` 格（原來那幾格的本地候選挪到下一頁），
//! 沒到、沒詞就什麼都不變：不預留、不畫佔位，頁面上只會「多出幾個詞」這一種變化。雲端詞只在第一頁，翻頁之後全是本地候選。
//! 這是展示規則不是排序規則，但 CLI 與各平臺殼都要用同一套，所以放在 Core。

mod candidate_layout;
mod cell;
mod grid;

pub use candidate_layout::CandidateLayout;
pub use cell::Cell;
pub use grid::{GRID_ROWS, Grid, MAX_CELL_EMS};
