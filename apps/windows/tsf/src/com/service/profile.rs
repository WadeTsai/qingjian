//! 把本執行緒用的語言 profile（簡 / 繁）報給 Server：連上時查一次，之後在兩個 profile 之間切換再報。
//! Server 開會話時按簡體算，所以連上時只有繁體才要發；切換時兩邊都發。

use super::TextService_Impl;
use crate::com::log::log;
use crate::com::profile;

impl TextService_Impl {
    /// 剛連上 Server：查當前 profile，是繁體就報。查不到按簡體。
    pub(super) fn report_profile(&self) {
        let traditional = profile::active_traditional().unwrap_or(false);
        self.profile_traditional.set(Some(traditional));
        if traditional {
            self.send_profile(traditional);
        }
    }

    /// 執行緒切到了青簡的另一個 profile；與上次報的一樣就不發。
    pub(super) fn adopt_profile(&self, traditional: bool) {
        if self.profile_traditional.get() == Some(traditional) {
            return;
        }
        self.profile_traditional.set(Some(traditional));
        self.send_profile(traditional);
    }

    fn send_profile(&self, traditional: bool) {
        // 沒連著 Server 就不發：下次 `connect` 會重新查一遍再報。
        let mut engine = self.engine.borrow_mut();
        let Some(client) = engine.as_mut() else {
            return;
        };
        let script = if traditional { "繁體（中文(台灣)）" } else { "簡體" };
        match client.profile(traditional) {
            Ok(()) => log(&format!("語言 profile：{script}")),
            Err(error) => log(&format!("報語言 profile 失敗: {error}")),
        }
    }
}
