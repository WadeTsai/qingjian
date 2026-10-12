//! 連 Server：啟用 / 獲焦時開會話，失敗按 [`RECONNECT_INTERVAL`] 退避重試；轉發出錯就斷開、下一鍵重連。

use std::time::Instant;

use super::launch;
use super::{RECONNECT_INTERVAL, TextService_Impl};
use crate::client::EngineClient;
use crate::client::pipe::connect_default;
use crate::com::log::log;

impl TextService_Impl {
    /// 連 Server 並開會話（會話號見 [`crate::com::session_id`]，帶上宿主 exe 名）。
    pub(super) fn connect(&self) {
        let session = crate::com::session_id();
        let app = crate::com::host_app_name();
        let connected = connect_default()
            .map_err(|e| e.to_string())
            .and_then(|stream| EngineClient::open(stream, session, app).map_err(|e| e.to_string()));
        match connected {
            Ok((client, input)) => {
                *self.engine.borrow_mut() = Some(client);
                self.last_connect_failure.set(None);
                log("已連上 Server");
                // 按鍵行為設定隨 `OpenSession` 的回包一起下來（DLL 不讀配置檔案，AppContainer 裡讀不到）。
                self.apply_input_settings(input);
                // Server 開會話按「中文(台灣)」的青簡（拼音）算；本執行緒用的是青簡注音就告訴它。
                self.report_profile();
            }
            Err(error) => {
                self.last_connect_failure.set(Some(Instant::now()));
                log(&format!(
                    "連 Server 失敗（qingjian-server 沒起？）: {error}"
                ));
                // Server 只在登入時由「啟動」資料夾拉起，中途掛了以前只能等下次登入；
                // 這裡自己起一次（程序內冷卻 + 跨程序互斥體，不會砸出一串 Server）。
                if launch::launch_server() {
                    // 不等 RECONNECT_INTERVAL：Server 一百多毫秒就監聽管道，下一鍵就該連上
                    self.last_connect_failure.set(None);
                }
            }
        }
    }

    /// 沒連上就重連一次（距上次失敗不到 [`RECONNECT_INTERVAL`] 則跳過）。返回此刻是否連著。
    pub(super) fn ensure_connected(&self) -> bool {
        if self.engine.borrow().is_some() {
            return true;
        }
        let recently_failed = self
            .last_connect_failure
            .get()
            .is_some_and(|at| at.elapsed() < RECONNECT_INTERVAL);
        if recently_failed {
            return false;
        }
        self.connect();
        let connected = self.engine.borrow().is_some();
        // 重連上的多半是重啟過的 Server，它不知道當前模式：前臺這邊報一次，成為全域性模式。
        if connected && self.shared.foreground() {
            self.report_mode();
        }
        connected
    }

    /// 轉發失敗後斷開，下一鍵重連。
    pub(super) fn disconnect(&self) {
        *self.engine.borrow_mut() = None;
        self.last_connect_failure.set(None);
        self.profile_flags.set(None);
        self.shared.end_composing();
    }
}
