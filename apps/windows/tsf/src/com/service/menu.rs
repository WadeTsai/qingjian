//! 工作列「中 / 英」圖示右鍵選單（見 [`crate::com::mode::menu`]）落到文本服務上。

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    ASFW_ANY, AllowSetForegroundWindow, GetForegroundWindow, GetWindowThreadProcessId,
};

use qingjian_platform::protocol::IndicatorCommand;

use super::TextService_Impl;
use crate::com::log::log;
use crate::com::mode::menu::{self, MenuChoice, MenuState};

impl TextService_Impl {
    pub(super) fn show_indicator_menu(&self, point: POINT) {
        let Some(owner) = self.menu_owner() else {
            log("右鍵選單：找不到本執行緒的視窗，不彈");
            return;
        };
        // 先跟上全域性模式，選單上的勾與懸浮狀態條一致（懸浮條上剛點過、還沒到輪詢那一拍時）
        self.sync_mode_from_server();
        let english = self.mode_state.english();
        let indicator = self.indicator_state.get();
        let state = MenuState {
            english,
            english_enabled: self.mode_state.enabled(),
            full_width: if english {
                indicator.english_full_width_punctuation
            } else {
                indicator.full_width_punctuation
            },
            status_bar: indicator.status_bar,
            update_available: indicator.update_available,
        };
        match menu::track(owner, point, &state) {
            Some(MenuChoice::Mode { english }) if english != self.mode_state.english() => {
                self.set_english_mode(english);
            }
            Some(MenuChoice::Server(command)) => self.send_indicator(command),
            _ => {}
        }
    }

    fn send_indicator(&self, command: IndicatorCommand) {
        if matches!(
            command,
            IndicatorCommand::OpenSettings | IndicatorCommand::OpenDownload
        ) {
            // 設定程式 / 瀏覽器由 Server 起；前臺權在點選單的這邊，讓出去它的窗口才能到前面
            let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
        }
        match self.engine.borrow_mut().as_mut() {
            Some(client) => {
                if let Err(error) = client.indicator(command) {
                    log(&format!("右鍵選單發給 Server 失敗: {error}"));
                }
            }
            None => log("右鍵選單：沒連上 Server"),
        }
    }

    /// 選單要掛在本執行緒的視窗上：先取焦點輸入框所在視窗，沒有再看前臺視窗是不是本執行緒的。
    fn menu_owner(&self) -> Option<HWND> {
        let from_context = self
            .thread_mgr
            .borrow()
            .as_ref()
            .and_then(|thread_mgr| unsafe {
                let view = thread_mgr
                    .GetFocus()
                    .ok()?
                    .GetTop()
                    .ok()?
                    .GetActiveView()
                    .ok()?;
                view.GetWnd().ok()
            });
        from_context
            .or_else(|| {
                let foreground = unsafe { GetForegroundWindow() };
                let thread = unsafe { GetWindowThreadProcessId(foreground, None) };
                (thread == unsafe { GetCurrentThreadId() }).then_some(foreground)
            })
            .filter(|hwnd| !hwnd.is_invalid())
    }
}
