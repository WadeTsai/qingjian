//! 編輯會話：TSF 不允許直接改文件，要經 `RequestEditSession` 申請、在回撥裡拿著 edit cookie 讀寫。
//! 寫組句 / 上屏在 [`update`]，讀選區（翻譯選中文字）在 [`selection`]，讀游標前文與輸入框私密判定（本地整句模型 / 密碼框）在 [`surrounding`]，候選視窗的定位錨點在 [`anchor`]。

mod anchor;
mod selection;
mod surrounding;
mod update;

pub(crate) use self::anchor::{anchor_rect, caret_rect};
pub(crate) use self::selection::request_selection;
pub(crate) use self::surrounding::{InputContext, input_context};
pub(crate) use self::update::request_update;
