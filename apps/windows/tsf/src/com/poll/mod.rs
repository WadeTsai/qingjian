//! 輪詢定時器：組句期間每隔一小段時間向 Server 拉一次非同步結果（雲端候選 / 整句補全）；
//! 沒在組句、本執行緒在前臺時，隔幾拍向 Server 取一次全域性中英模式（別的應用、懸浮狀態條可能切過）。
//! 雲端結果幾百毫秒後才回，那時往往沒有新按鍵來「順手收一次」，所以在 TSF 執行緒上掛一個 `WM_TIMER`；
//! 傳輸仍是一問一答。定時器掛在隱藏的訊息視窗上，與按鍵同在 STA 訊息泵上跑。回撥上下文在 [`context`]。
//! 滑鼠點候選視窗上屏的文本也是這樣拉回來的（Server 不能直接寫文件），拉到就開編輯會話寫進去。

mod context;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use windows::Win32::Foundation::{E_FAIL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClassNameW, GetForegroundWindow,
    GetWindowThreadProcessId, HWND_MESSAGE, KillTimer, SetTimer, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_TIMER, WNDCLASSEXW,
};
use windows::core::{Error, PCWSTR, Result, w};

use qingjian_platform::protocol::Frame;

use self::context::PollContext;
use super::composition::{Shared, preedit_string};
use super::edit::request_update;
use super::log::log;
use super::service::SharedClient;
use super::window_class::WindowClass;

const CLASS_NAME: PCWSTR = w!("QingjianPollWindow");
static CLASS: WindowClass = WindowClass::new();

const TIMER_ID: usize = 1;
const INTERVAL_MS: u32 = 80;

/// 沒在組句時每幾拍問一次狀態條的切模式請求（320 ms 一次，點了狀態條肉眼看不出延遲）。
/// 按鍵行為設定（切換鍵、內建英文模式）也跟著這一拍取回，所以設定改完同樣是約 320 ms 生效。
const MODE_SYNC_EVERY: u32 = 4;

thread_local! {
    /// 本執行緒活著的定時器：訊息視窗 → 回撥上下文。查不到（已析構）就忽略這一拍。
    static TIMERS: RefCell<HashMap<isize, Rc<PollContext>>> = RefCell::new(HashMap::new());
}

/// 承載輪詢定時器的隱藏訊息視窗；`Drop` 裡停表、銷燬視窗、登出上下文。
pub(crate) struct PollTimer {
    hwnd: HWND,
}

impl PollTimer {
    /// 失敗返回 `Err`，呼叫方降級為只在按鍵時收雲結果。
    pub(crate) fn new(engine: SharedClient, shared: Rc<Shared>, client_id: u32) -> Result<Self> {
        CLASS.ensure(|| WNDCLASSEXW {
            lpfnWndProc: Some(wndproc),
            hInstance: super::dll_instance(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        })?;
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                CLASS_NAME,
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(super::dll_instance()),
                None,
            )?
        };
        let timer = Self { hwnd };
        if unsafe { SetTimer(Some(hwnd), TIMER_ID, INTERVAL_MS, None) } == 0 {
            return Err(Error::from(E_FAIL));
        }
        TIMERS.with(|timers| {
            timers.borrow_mut().insert(
                hwnd.0 as isize,
                Rc::new(PollContext {
                    engine,
                    shared,
                    ticks: Cell::new(0),
                    client_id,
                }),
            )
        });
        Ok(timer)
    }
}

impl Drop for PollTimer {
    fn drop(&mut self) {
        TIMERS.with(|timers| timers.borrow_mut().remove(&(self.hwnd.0 as isize)));
        unsafe {
            let _ = KillTimer(Some(self.hwnd), TIMER_ID);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_TIMER {
        // 先 clone 出來放開表的借用。
        let context = TIMERS.with(|timers| timers.borrow().get(&(hwnd.0 as isize)).cloned());
        if let Some(context) = context {
            // 從訊息泵調進來：panic 不能越過 FFI。
            let _ = catch_unwind(AssertUnwindSafe(|| poll_once(&context)));
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// 組句中或翻譯評審中拉雲結果；否則前臺時隔幾拍問一次切模式（順路取回按鍵行為設定）。引擎正被按鍵處理借用時跳過這一拍；連線壞了斷開。
fn poll_once(context: &PollContext) {
    let tick = context.ticks.get().wrapping_add(1);
    context.ticks.set(tick);
    let translating = context.shared.translating();
    if !context.shared.composing() && !translating {
        if tick.is_multiple_of(MODE_SYNC_EVERY) && in_foreground(context) {
            sync_mode(context);
        }
        return;
    }
    let Ok(mut guard) = context.engine.try_borrow_mut() else {
        return;
    };
    let Some(client) = guard.as_mut() else {
        return;
    };
    match client.poll() {
        Ok((frame, Some(commit))) => {
            drop(guard);
            commit_clicked(context, &frame, commit);
        }
        Ok((frame, None)) => {
            // 翻譯評審時回空幀 = 翻譯已在 Server 側結束（雲端沒給譯文）。
            if translating && frame.is_empty() {
                drop(guard);
                context.shared.set_translating(false);
                context.shared.hide_candidates();
            }
        }
        Err(error) => {
            log(&format!("雲聯想輪詢失敗，斷開，下一鍵重連: {error}"));
            *guard = None;
            context.shared.end_composing();
        }
    }
}

/// 滑鼠點候選視窗上屏：與按鍵結果一樣經非同步編輯會話寫進文件，剩下的拼音接著組句。
fn commit_clicked(context: &PollContext, frame: &Frame, commit: String) {
    let Some(document) = context.shared.last_context() else {
        log(&format!("點選上屏沒有上下文，丟棄: {commit:?}"));
        return;
    };
    // 「只在候選視窗」模式應用裡不放行內拼音，與按鍵路徑一致
    let preedit = if frame.preedit_mode.inline() {
        preedit_string(frame)
    } else {
        String::new()
    };
    context.shared.set_composing(!frame.is_empty());
    log(&format!("點選上屏: {commit:?} preedit={preedit:?}"));
    if let Err(error) = request_update(
        &document,
        context.client_id,
        context.engine.clone(),
        context.shared.clone(),
        Some(commit),
        preedit,
    ) {
        log(&format!("點選上屏的編輯會話沒被受理: {error}"));
    }
}

/// 本程序是不是在前臺：當場看前臺視窗屬於誰，不只信執行緒焦點標記（後臺程序的標記可能一直不清，
/// 它來取模式會讓 Server 以為青簡仍是當前輸入法）。UWP 應用的前臺視窗在 ApplicationFrameHost 程序，那時退回看標記。
fn in_foreground(context: &PollContext) -> bool {
    let window = unsafe { GetForegroundWindow() };
    if window.is_invalid() {
        return false;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
    pid == unsafe { GetCurrentProcessId() } || (is_uwp_frame(window) && context.shared.foreground())
}

/// UWP 應用的框架視窗（類名 `ApplicationFrameWindow`）。
fn is_uwp_frame(window: HWND) -> bool {
    let mut name = [0u16; 32];
    let len = unsafe { GetClassNameW(window, &mut name) };
    usize::try_from(len)
        .is_ok_and(|len| String::from_utf16_lossy(&name[..len]) == "ApplicationFrameWindow")
}

/// 取一次全域性中英模式，順路取回 Server 下發的按鍵行為設定。切模式要落定組句、會借引擎，
/// 所以先放掉借用再切。
fn sync_mode(context: &PollContext) {
    let Ok(mut guard) = context.engine.try_borrow_mut() else {
        return;
    };
    let Some(client) = guard.as_mut() else {
        // 沒連著（Server 起得晚 / 重啟過）：不等下一鍵，這一拍就補連（自帶退避）。
        drop(guard);
        super::service::on_reconnect_tick();
        return;
    };
    let reply = match client.sync_mode() {
        Ok(reply) => reply,
        Err(error) => {
            log(&format!("同步中英模式失敗，斷開，下一鍵重連: {error}"));
            *guard = None;
            return;
        }
    };
    drop(guard);
    // 按鍵行為設定每一拍都帶（DLL 不讀配置檔案），切換鍵與內建英文模式開關改完靠它生效。
    super::service::on_input_settings(reply.input);
    super::service::on_indicator_state(reply.indicator);
    if let Some(english) = reply.english {
        super::service::on_mode_sync(english);
    }
}
