//! 「輔碼」頁觸發鍵錄製的狀態，掛在 [`Settings`](super::Settings) 上：點「錄製」後等使用者按一個鍵。

/// 觸發鍵錄製狀態。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Recorder {
    /// 沒在錄，控制元件顯示當前觸發鍵。
    #[default]
    Idle,

    /// 在等使用者按下一個鍵。
    Waiting {
        /// 第幾輪等待，從 1 起：錄製框拿它當 `KeyedView` 標識，一輪一代新框——
        /// 上一輪敲進來的字元不會被 Reactor 重寫回宣告值，只能靠重建清掉。
        attempt: u64,
    },
}

impl Recorder {
    /// 進入下一輪等待（錄製框跟著換代）。
    pub(crate) fn waiting(self) -> Self {
        let attempt = match self {
            Self::Idle => 0,
            Self::Waiting { attempt } => attempt,
        };
        Self::Waiting {
            attempt: attempt + 1,
        }
    }
}
