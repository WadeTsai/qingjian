//! 一個語言 profile 決定的輸入行為：拼音側是不是注音（鍵盤排列由 Server 的配置定）。DLL 報給 Server 的就是這一位；
//! 繁體與否不再由 profile 決定——青簡只登記在「中文(台灣)」下，Windows 上一律出繁體。

/// profile 決定的輸入行為。`Default` 是「中文(台灣)」下的青簡（拼音）：按配置的方案。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProfileFlags {
    /// 不看 `[general] scheme`，拼音側按注音解鍵（排列仍看 `[general] zhuyin_layout`）。
    pub zhuyin: bool,
}

impl ProfileFlags {
    /// 日誌裡的說法。
    pub(crate) fn describe(self) -> &'static str {
        if self.zhuyin {
            "繁體 · 注音（中文(台灣) 的青簡注音）"
        } else {
            "繁體（中文(台灣) 的青簡）"
        }
    }
}
