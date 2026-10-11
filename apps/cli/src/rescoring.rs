//! 非同步重打分在 CLI 裡的等法：殼是停頓後請求、定時器輪詢，這裡沒有停頓，查詢完直接請求並阻塞等結果，再查一次。

use std::time::{Duration, Instant};

use qingjian_core::Engine;

/// 最多等多久。
const WAIT: Duration = Duration::from_secs(5);

/// 有整句路徑還沒拿到神經分就請求並等到分回來；返回是否等到了（等到了呼叫方該重新查詢）。
pub fn settle(engine: &mut Engine) -> bool {
    if !engine.rescoring_pending() || !engine.request_rescoring() {
        return false;
    }
    let started = Instant::now();
    while !engine.poll_rescoring() {
        if started.elapsed() > WAIT {
            tracing::warn!("等重打分超时");
            return false;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    true
}
