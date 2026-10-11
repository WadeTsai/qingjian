use std::time::Duration;

/// [`super::Engine::annotate`] 的統計。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnnotationReport {
    /// 嘗試標註的候選數。
    pub total: usize,

    /// 查到譯文的候選數。
    pub hits: usize,

    /// 查表總耗時。
    pub elapsed: Duration,
}
