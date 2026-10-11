//! 本地整句模型的兩個節拍：停鍵後的防抖（到點才把整句路徑送去後臺打分）與結果輪詢。
//! Server 沒有定時器，主迴圈按 [`RescoreState::next_deadline`] 給的時長等訊息，超時就來一次 `tick`；
//! 常數與 macOS 殼的 `RescoreMonitor`、Windows Server 的同名模組相同。

use std::time::{Duration, Instant};

/// 停鍵多久才請求重排：比一般的擊鍵間隔短，連著敲時不請求。
pub(super) const DEBOUNCE: Duration = Duration::from_millis(80);

/// 輪詢間隔：模型一次幾十毫秒。
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// 最長等多久；後臺執行緒卡住時兜底。
pub(super) const MAX_WAIT: Duration = Duration::from_secs(2);

/// 重排的進行態。
#[derive(Debug, Default)]
pub(crate) struct RescoreState {
    /// 最近一次緩衝變化的時間；有它表示在等防抖到點。
    wanted_since: Option<Instant>,

    /// 本輪請求發出的時間；有它表示在等後臺結果。
    polling_since: Option<Instant>,
}

impl RescoreState {
    /// 又敲了一鍵：重新計時。
    pub(super) fn schedule(&mut self) {
        self.wanted_since = Some(Instant::now());
    }

    /// 防抖到點了沒。
    pub(super) fn debounce_elapsed(&self) -> bool {
        self.wanted_since
            .is_some_and(|since| since.elapsed() >= DEBOUNCE)
    }

    /// 請求已發出：開始等結果。
    pub(super) fn start_polling(&mut self) {
        self.wanted_since = None;
        self.polling_since = Some(Instant::now());
    }

    pub(super) fn polling(&self) -> bool {
        self.polling_since.is_some()
    }

    /// 等結果等太久了。
    pub(super) fn expired(&self) -> bool {
        self.polling_since
            .is_some_and(|since| since.elapsed() > MAX_WAIT)
    }

    /// 什麼都不等。
    pub(super) fn stop(&mut self) {
        self.wanted_since = None;
        self.polling_since = None;
    }

    /// 下一次該來 `tick` 的時長；什麼都不等為 `None`。
    pub(super) fn next_deadline(&self) -> Option<Duration> {
        if self.polling_since.is_some() {
            return Some(POLL_INTERVAL);
        }
        self.wanted_since
            .map(|since| DEBOUNCE.saturating_sub(since.elapsed()))
    }
}
