/// 一類來源的計數。
#[derive(Debug, Default, Clone, Copy)]
pub struct Tally {
    /// 日誌裡這類上屏的總數。
    pub total: usize,

    /// 現在的首選就是當時選的。
    pub top1: usize,

    /// 在第 2 到第 5 位。
    pub top5: usize,

    /// 在第 6 位之後。
    pub found_later: usize,

    /// 當時選的詞現在根本不在候選裡（詞庫變了、刪過詞、雲端詞）。
    pub missing: usize,

    /// 作用域現在切不動（雙拼方案 / 模式鍵變了）。
    pub unparsable: usize,

    /// 找到時的名次之和（算平均名次）。
    pub rank_sum: usize,

    /// 當時拼寫糾錯生效的條數。
    pub corrected_then: usize,

    /// 其中現在仍然糾錯的條數。
    pub corrected_now: usize,

    /// 當時就命中首選的條數（日誌裡 `index == 0`），與「現在」無關，是隱式訊號與重排對比的分母。
    pub then_top1: usize,

    /// 當時命中首選的那些：翻頁數之和、耗時之和（毫秒）。
    pub hit_pages: usize,
    pub hit_ms: u64,

    /// 當時沒命中首選的那些：翻頁數之和、耗時之和。
    pub miss_pages: usize,
    pub miss_ms: u64,

    /// 當時經過神經重排的條數，與其中當時命中首選的條數。
    pub rescored_then: usize,
    pub rescored_then_top1: usize,
}

impl Tally {
    /// 從日誌裡的一條記下當時的情況（命中、翻頁、耗時、重排），回放前調。
    pub fn note_logged(&mut self, commit: &qingjian_core::CommitEntry) {
        let hit = commit.index == Some(0);
        if hit {
            self.then_top1 += 1;
            self.hit_pages += commit.pages as usize;
            self.hit_ms += commit.ms;
        } else {
            self.miss_pages += commit.pages as usize;
            self.miss_ms += commit.ms;
        }
        if commit.rescored {
            self.rescored_then += 1;
            self.rescored_then_top1 += usize::from(hit);
        }
    }

    /// 當時命中 / 沒命中兩組的平均翻頁數與平均耗時；日誌裡沒這兩個欄位（舊格式）時全是 0，返回 `None`。
    pub fn implicit_signal(&self) -> Option<((f64, f64), (f64, f64))> {
        let misses = self.total.saturating_sub(self.then_top1);
        if self.hit_ms + self.miss_ms == 0 || self.then_top1 == 0 || misses == 0 {
            return None;
        }
        let hit = (
            self.hit_pages as f64 / self.then_top1 as f64,
            self.hit_ms as f64 / self.then_top1 as f64,
        );
        let miss = (
            self.miss_pages as f64 / misses as f64,
            self.miss_ms as f64 / misses as f64,
        );
        Some((hit, miss))
    }
}

impl Tally {
    /// 現在能評的條數（找到 + 沒找到，不含切不動的）。
    pub fn evaluated(&self) -> usize {
        self.total - self.unparsable
    }

    pub fn found(&self) -> usize {
        self.top1 + self.top5 + self.found_later
    }

    pub fn mean_rank(&self) -> Option<f64> {
        (self.found() > 0).then(|| self.rank_sum as f64 / self.found() as f64)
    }
}
