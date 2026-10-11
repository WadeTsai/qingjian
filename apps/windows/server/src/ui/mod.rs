//! Server 程序內的 UI 執行緒：候選視窗與懸浮狀態條都在這條執行緒上自繪（普通置頂窗會被商店 / 工作列搜尋
//! 這些高 z-band 宿主蓋住，只有本程序配合 uiAccess 簽名才能蓋過）。
//!
//! HWND 執行緒親和：Router 在工人執行緒上產出內容，經通道 + `PostThreadMessageW` 喚醒交給 UI 執行緒應用。
//! 執行緒控制代碼是 [`UiHandle`]，命令在 [`command`]，候選視窗在 [`candidates`]，狀態條在 [`status`]，分層視窗合成在 [`layered`]。

mod candidates;
mod command;
mod layered;
mod monitor;
mod painter;
mod status;
mod window_class;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use windows::Win32::Foundation::{E_FAIL, HINSTANCE, LPARAM, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    ASFW_ANY, AllowSetForegroundWindow, DispatchMessageW, GetMessageW, MSG, PostThreadMessageW,
    TranslateMessage, WM_APP,
};
use windows::core::{Error, Result};

use qingjian_platform::protocol::{Frame, ScreenRect};
use qingjian_render::Mode;

pub use self::candidates::CandidateClicks;
use self::candidates::CandidateWindow;
use self::command::UiCommand;
use self::painter::{Painter, SharedPainter};
use self::status::StatusBar;
use crate::dispatch::{CandidateSink, RenderSettings, StatusEvent, StatusSink, StatusView};

/// 狀態條上的操作（點格子 / 拖動結束）回給 Router 的回撥，UI 執行緒上調。
pub type StatusEvents = Box<dyn Fn(StatusEvent) + Send>;

/// 喚醒 UI 執行緒去排空命令佇列的執行緒訊息。
const WM_WAKE: u32 = WM_APP;

/// UI 執行緒控制代碼：發命令 = 投進通道 + 一條 `WM_WAKE`。執行緒 detached，隨程序存活。
#[derive(Clone)]
pub struct UiHandle {
    /// 命令通道的傳送端。
    sender: Sender<UiCommand>,

    /// UI 執行緒 id，`PostThreadMessageW` 用。
    thread_id: u32,
}

impl UiHandle {
    /// 起 UI 執行緒並等它建好候選視窗。失敗返回 `Err`，呼叫方退化為不畫。
    pub fn spawn(on_status: StatusEvents, on_click: CandidateClicks) -> Result<Self> {
        // 用 Option<u32> 而非 Result 回報，免得 windows Error 跨執行緒。
        let (ready_tx, ready_rx) = mpsc::channel::<Option<u32>>();
        let (command_tx, command_rx) = mpsc::channel::<UiCommand>();
        thread::Builder::new()
            .name("qingjian-candidates".to_owned())
            .spawn(move || run(command_rx, &ready_tx, on_status, on_click))
            .map_err(|_| Error::from(E_FAIL))?;
        match ready_rx.recv() {
            Ok(Some(thread_id)) => Ok(Self {
                sender: command_tx,
                thread_id,
            }),
            _ => Err(Error::from(E_FAIL)),
        }
    }

    /// 執行緒已退出（通道斷）時靜默丟棄。
    fn post(&self, command: UiCommand) {
        if self.sender.send(command).is_ok() {
            let _ = unsafe { PostThreadMessageW(self.thread_id, WM_WAKE, WPARAM(0), LPARAM(0)) };
        }
    }
}

impl CandidateSink for UiHandle {
    fn show(&self, frame: Frame, mode: Mode, rect: ScreenRect) {
        self.post(UiCommand::Show(Box::new((frame, mode, rect))));
    }

    fn hide(&self) {
        self.post(UiCommand::Hide);
    }

    fn configure(&self, settings: RenderSettings) {
        self.post(UiCommand::Configure(settings));
    }
}

impl StatusSink for UiHandle {
    fn show_status(&self, view: StatusView) {
        self.post(UiCommand::StatusShow(Box::new(view)));
    }

    fn hide_status(&self) {
        self.post(UiCommand::StatusHide);
    }

    fn open_settings(&self) {
        open_settings();
    }

    fn open_download(&self) {
        let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
        let opened = std::process::Command::new("explorer")
            .arg(qingjian_update::DOWNLOAD_URL)
            .spawn();
        if let Err(error) = opened {
            tracing::warn!(%error, "開啟下載頁失敗");
        }
    }
}

/// 起與本 exe 同目錄的設定程式。設定程式已開時由新例項把它帶到前臺，得先把前臺權讓出去。
pub(crate) fn open_settings() {
    let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    let exe = std::env::current_exe().map(|exe| exe.with_file_name("qingjian-settings.exe"));
    let spawned = exe.and_then(|exe| std::process::Command::new(exe).spawn());
    if let Err(error) = spawned {
        tracing::warn!(%error, "開啟設定程式失敗");
    }
}

/// 本程序 exe 的模組控制代碼（註冊視窗類 / 建視窗用）。
pub(super) fn module_handle() -> HINSTANCE {
    let module = unsafe { GetModuleHandleW(None) }.unwrap_or_default();
    HINSTANCE(module.0)
}

/// UI 執行緒主體：建視窗、報回執行緒 id、跑訊息迴圈。
fn run(
    commands: Receiver<UiCommand>,
    ready: &Sender<Option<u32>>,
    on_status: StatusEvents,
    on_click: CandidateClicks,
) {
    // 按物理畫素定位，與應用報來的組句螢幕矩形對齊；已設過會失敗，忽略。
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let thread_id = unsafe { GetCurrentThreadId() };
    // 裝上時隨 Configure 命令建。
    let painter: SharedPainter = Rc::new(RefCell::new(None));
    // 先建視窗再報 id：建視窗順帶建起本執行緒的訊息佇列，之後 PostThreadMessageW 才有處可投。
    let window = match CandidateWindow::new(painter.clone(), on_click) {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "建候選視窗失敗，Server 將不顯示候選框");
            let _ = ready.send(None);
            return;
        }
    };
    let status = match StatusBar::new(on_status, painter.clone()) {
        Ok(status) => Some(status),
        Err(error) => {
            tracing::error!(%error, "建懸浮狀態條失敗，將不顯示狀態條");
            None
        }
    };
    if ready.send(Some(thread_id)).is_err() {
        return;
    }
    let mut msg = MSG::default();
    loop {
        let got = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if got.0 <= 0 {
            break;
        }
        if window.is_animation_timer(&msg) {
            window.animation_frame();
            continue;
        }
        if msg.message == WM_WAKE {
            // 一次喚醒排空整個佇列，保住 Hide→Show 的先後。
            while let Ok(command) = commands.try_recv() {
                apply(&window, status.as_ref(), &painter, command);
            }
            continue;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn apply(
    window: &CandidateWindow,
    status: Option<&StatusBar>,
    painter: &SharedPainter,
    command: UiCommand,
) {
    match command {
        UiCommand::Show(payload) => {
            let (frame, mode, rect) = *payload;
            window.set_content(&frame, mode);
            window.show(to_win_rect(rect));
        }
        UiCommand::Hide => window.hide(),
        UiCommand::StatusShow(view) => {
            if let Some(status) = status {
                status.update(*view);
            }
        }
        UiCommand::StatusHide => {
            if let Some(status) = status {
                status.hide();
            }
        }
        UiCommand::Configure(settings) => Painter::configure(painter, &settings),
    }
}

fn to_win_rect(rect: ScreenRect) -> RECT {
    RECT {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}
