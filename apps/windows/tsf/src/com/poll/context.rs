//! 輪詢定時器每一拍回撥要用的東西：按訊息視窗控制代碼從執行緒表裡查出來。

use std::cell::Cell;
use std::rc::Rc;

use crate::com::composition::Shared;
use crate::com::service::SharedClient;

pub(super) struct PollContext {
    /// 引擎層：拉雲結果 / 問切模式請求。
    pub(super) engine: SharedClient,

    /// 組句狀態：在不在組句 / 翻譯評審 / 前臺。
    pub(super) shared: Rc<Shared>,

    /// 拍數計數，給 [`MODE_SYNC_EVERY`](super::MODE_SYNC_EVERY) 取模。
    pub(super) ticks: Cell<u32>,

    /// 本服務的 TSF client id：點選上屏要申請編輯會話。
    pub(super) client_id: u32,
}
