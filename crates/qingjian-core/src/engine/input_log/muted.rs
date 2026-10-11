use super::{InputLogEntry, InputLogger};

/// 套在殼裝配的輸入日誌外面的一層：私密輸入期間一條都不記（條目裡有敲的鍵與上屏的字），見 [`crate::Engine::set_private`]。
pub(in crate::engine) struct MutedLogger {
    inner: Box<dyn InputLogger>,

    /// 私密中：`record` 直接丟。
    muted: bool,
}

impl MutedLogger {
    pub(in crate::engine) fn new(inner: Box<dyn InputLogger>) -> Self {
        Self {
            inner,
            muted: false,
        }
    }

    pub(in crate::engine) fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    pub(in crate::engine) fn replace(&mut self, inner: Box<dyn InputLogger>) {
        self.inner = inner;
    }

    pub(in crate::engine) fn inner_mut(&mut self) -> &mut dyn InputLogger {
        self.inner.as_mut()
    }
}

impl InputLogger for MutedLogger {
    fn record(&mut self, entry: InputLogEntry) {
        if !self.muted {
            self.inner.record(entry);
        }
    }

    fn flush(&mut self) {
        self.inner.flush();
    }

    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }
}
