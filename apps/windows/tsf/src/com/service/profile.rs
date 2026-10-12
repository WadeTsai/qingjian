//! 把本執行緒用的語言 profile（繁體拼音 / 繁體注音）報給 Server：連上時查一次，之後在 profile 之間切換再報。
//! Server 開會話時按預設 profile（拼音、按配置方案）算，所以連上時只有不是預設的才要發；切換時都發。
//! 繁體這一位一律報 `true`：青簡只登記在「中文(台灣)」下，Windows 上不再有簡體輸出。

use super::TextService_Impl;
use crate::com::log::log;
use crate::com::profile::{self, ProfileFlags};

impl TextService_Impl {
    /// 剛連上 Server：查當前 profile，不是預設的就報。查不到按預設。
    pub(super) fn report_profile(&self) {
        let flags = profile::active_flags().unwrap_or_default();
        self.profile_flags.set(Some(flags));
        if flags != ProfileFlags::default() {
            self.send_profile(flags);
        }
    }

    /// 執行緒切到了青簡的另一個 profile；與上次報的一樣就不發。
    pub(super) fn adopt_profile(&self, flags: ProfileFlags) {
        if self.profile_flags.get() == Some(flags) {
            return;
        }
        self.profile_flags.set(Some(flags));
        self.send_profile(flags);
    }

    fn send_profile(&self, flags: ProfileFlags) {
        // 沒連著 Server 就不發：下次 `connect` 會重新查一遍再報。
        let mut engine = self.engine.borrow_mut();
        let Some(client) = engine.as_mut() else {
            return;
        };
        match client.profile(true, flags.zhuyin) {
            Ok(()) => log(&format!("語言 profile：{}", flags.describe())),
            Err(error) => log(&format!("報語言 profile 失敗: {error}")),
        }
    }
}
