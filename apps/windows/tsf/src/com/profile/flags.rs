//! 一個語言 profile 決定的輸入行為：出不出繁體、拼音側是不是注音（鍵盤排列由 Server 的配置定）。DLL 報給 Server 的就是這兩位。

/// profile 決定的輸入行為。`Default` 是「中文(簡體)」下的青簡：簡體、按配置的方案。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProfileFlags {
    /// 不看 `[general] traditional` 就出繁體。
    pub traditional: bool,

    /// 不看 `[general] scheme`，拼音側按注音解鍵（排列仍看 `[general] zhuyin_layout`）。
    pub zhuyin: bool,
}

impl ProfileFlags {
    /// 日誌裡的說法。
    pub(crate) fn describe(self) -> &'static str {
        match (self.traditional, self.zhuyin) {
            (true, true) => "繁體 · 注音（中文(台灣) 的青簡注音）",
            (true, false) => "繁體（中文(台灣) 的青簡）",
            (false, true) => "簡體 · 注音",
            (false, false) => "簡體",
        }
    }
}
