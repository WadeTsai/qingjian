//! 各頁操作後的臨時提示，掛在 [`Settings`](super::Settings) 上：一行灰字（匯入統計等）與一行紅字（失敗原因）。

/// 上一次操作的提示文案；切頁時清掉。
#[derive(Default)]
pub(crate) struct Notice {
    /// 結果說明（匯入統計、匯入成功等），頁面底部灰字顯示。
    pub(crate) note: Option<String>,

    /// 失敗原因，頁面底部紅字顯示。
    pub(crate) error: Option<String>,
}

impl Notice {
    /// 記下一次成功：寫說明、清掉上一次的錯誤。
    pub(crate) fn succeed(&mut self, note: String) {
        self.note = Some(note);
        self.error = None;
    }

    /// 記下一次失敗：寫錯誤、清掉上一次的說明。
    pub(crate) fn fail(&mut self, error: String) {
        self.error = Some(error);
        self.note = None;
    }

    /// 清空兩行（換頁、成功的設定改動都調）。
    pub(crate) fn clear(&mut self) {
        self.note = None;
        self.error = None;
    }
}
