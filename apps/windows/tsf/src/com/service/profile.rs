//! 把本线程用的语言 profile（简 / 繁）报给 Server：连上时查一次，之后在两个 profile 之间切换再报。
//! Server 开会话时按简体算，所以连上时只有繁体才要发；切换时两边都发。

use super::TextService_Impl;
use crate::com::log::log;
use crate::com::profile;

impl TextService_Impl {
    /// 刚连上 Server：查当前 profile，是繁体就报。查不到按简体。
    pub(super) fn report_profile(&self) {
        let traditional = profile::active_traditional().unwrap_or(false);
        self.profile_traditional.set(Some(traditional));
        if traditional {
            self.send_profile(traditional);
        }
    }

    /// 线程切到了青简的另一个 profile；与上次报的一样就不发。
    pub(super) fn adopt_profile(&self, traditional: bool) {
        if self.profile_traditional.get() == Some(traditional) {
            return;
        }
        self.profile_traditional.set(Some(traditional));
        self.send_profile(traditional);
    }

    fn send_profile(&self, traditional: bool) {
        // 没连着 Server 就不发：下次 `connect` 会重新查一遍再报。
        let Some(sent) = self
            .engine
            .borrow_mut()
            .as_mut()
            .map(|client| client.profile(traditional))
        else {
            return;
        };
        match sent {
            Ok(()) => log(&format!(
                "语言 profile：{}",
                if traditional { "繁体（中文(台灣)）" } else { "简体" }
            )),
            Err(error) => log(&format!("报语言 profile 失败: {error}")),
        }
    }
}
