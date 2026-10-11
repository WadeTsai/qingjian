use std::borrow::Cow;
use std::io::{self, Write};

use super::mask;

/// 包在日誌檔案外面的寫入器：每次寫入先過一遍 [`mask`]。
/// 要放在 `tracing_appender::non_blocking` 裡面——那裡一條日誌是一次完整的寫入，金鑰不會被切成兩半。
pub struct MaskingWriter<W> {
    inner: W,
}

impl<W> MaskingWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for MaskingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match mask(&String::from_utf8_lossy(buf)) {
            Cow::Borrowed(_) => self.inner.write_all(buf)?,
            Cow::Owned(masked) => self.inner.write_all(masked.as_bytes())?,
        }
        // 寫進去的位元組數可能變少了，對呼叫方仍按原長度算寫完
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
