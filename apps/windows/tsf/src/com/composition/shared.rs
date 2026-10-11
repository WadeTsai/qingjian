use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::UI::TextServices::{ITfComposition, ITfContext};

use crate::com::service::SharedClient;

/// `TextService`、編輯會話、組句 sink、輪詢定時器之間共享的組句狀態（STA 單執行緒，`Rc` 傳遞）。
pub(crate) struct Shared {
    /// 當前活動的組句，跨按鍵存活。
    composition: RefCell<Option<ITfComposition>>,

    /// Server 上次回的幀非空；決定 `OnTestKeyDown` 要不要吃功能鍵。
    composing: Cell<bool>,

    /// 這一段的輸入框狀態（私密與否、游標前文）已經報給 Server 了。
    /// 行內模式下靠「組句剛起」判斷，`preedit = window` 模式應用裡沒有組句，得另記一個標記。
    context_reported: Cell<bool>,

    /// 「翻譯選中文字」評審進行中：所有鍵交給 Server 定接受 / 取消，輪詢定時器照常拉雲端譯文。
    translating: Cell<bool>,

    /// 最近一次收鍵的文件上下文；失焦 / 停用回撥不帶上下文，落定拼音要用它。
    last_context: RefCell<Option<ITfContext>>,

    /// 組句被應用強行終止過：拼音已成普通文本，但 Server 的緩衝還在，下次說話前先讓它清空。
    server_stale: Cell<bool>,

    /// 本執行緒當前有鍵盤焦點（`OnSetFocus`）；輪詢定時器只在前臺時問狀態條的切模式請求。
    foreground: Cell<bool>,

    /// 與 `TextService` 共用的引擎客戶端；DLL 側結束組句時要通知 Server 收候選視窗（它無從知曉）。
    client: SharedClient,
}

impl Shared {
    pub(crate) fn new(client: SharedClient) -> Rc<Self> {
        Rc::new(Self {
            composition: RefCell::new(None),
            composing: Cell::new(false),
            context_reported: Cell::new(false),
            translating: Cell::new(false),
            last_context: RefCell::new(None),
            server_stale: Cell::new(false),
            foreground: Cell::new(false),
            client,
        })
    }

    pub(crate) fn foreground(&self) -> bool {
        self.foreground.get()
    }

    pub(crate) fn set_foreground(&self, value: bool) {
        self.foreground.set(value);
    }

    pub(crate) fn last_context(&self) -> Option<ITfContext> {
        self.last_context.borrow().clone()
    }

    pub(crate) fn set_last_context(&self, context: Option<ITfContext>) {
        *self.last_context.borrow_mut() = context;
    }

    pub(crate) fn take_server_stale(&self) -> bool {
        self.server_stale.replace(false)
    }

    pub(crate) fn composing(&self) -> bool {
        self.composing.get()
    }

    pub(crate) fn set_composing(&self, value: bool) {
        self.composing.set(value);
        // 組句結束：下一段重新報輸入框狀態。
        if !value {
            self.context_reported.set(false);
        }
    }

    pub(crate) fn context_reported(&self) -> bool {
        self.context_reported.get()
    }

    pub(crate) fn set_context_reported(&self, value: bool) {
        self.context_reported.set(value);
    }

    pub(crate) fn translating(&self) -> bool {
        self.translating.get()
    }

    pub(crate) fn set_translating(&self, value: bool) {
        self.translating.set(value);
    }

    /// 通知 Server 收起候選視窗。引擎正被別處藉著或沒連上時靜默跳過。
    pub(crate) fn hide_candidates(&self) {
        if let Ok(mut guard) = self.client.try_borrow_mut()
            && let Some(client) = guard.as_mut()
        {
            let _ = client.hide_candidates();
        }
    }

    pub(crate) fn has_composition(&self) -> bool {
        self.composition.borrow().is_some()
    }

    /// clone 出來再用，別把 `borrow()` 掛在 match 上（分支裡再 `borrow_mut` 會 panic）。
    pub(super) fn composition(&self) -> Option<ITfComposition> {
        self.composition.borrow().clone()
    }

    pub(super) fn set_composition(&self, composition: Option<ITfComposition>) {
        *self.composition.borrow_mut() = composition;
    }

    pub(super) fn take_composition(&self) -> Option<ITfComposition> {
        self.composition.borrow_mut().take()
    }

    /// 組句結束（應用終止組句 / 斷線 / 失焦上屏）：不再當作在組句，並讓 Server 收候選視窗。
    pub(crate) fn end_composing(&self) {
        self.composing.set(false);
        self.translating.set(false);
        self.hide_candidates();
    }

    /// 清掉一切本地組句狀態，不碰文件。
    pub(crate) fn reset(&self) {
        self.set_composition(None);
        self.set_last_context(None);
        self.end_composing();
    }

    /// 組句被應用強行終止：本地清掉，並記下 Server 的緩衝還沒清。
    pub(super) fn terminated(&self) {
        self.reset();
        self.server_stale.set(true);
    }
}
