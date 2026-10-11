use std::fmt;
use std::time::Duration;

use super::coverage::Coverage;

/// 整句評測報告。
#[derive(Debug, Default)]
pub struct Report {
    /// 抽出來的句子數（去重後）。
    pub extracted: usize,

    /// 有字查不到讀音、沒法轉拼音的句子數。
    pub untranscribable: usize,

    /// 評了的句子數。
    pub total: usize,

    /// 拼音現在切不動的句子數。
    pub unparsable: usize,

    /// 首選就是原句。
    pub top1: usize,

    /// 原句出現在前三 / 前五個候選裡（多給幾條整句候選值不值，看這兩個差多少）。
    pub top3: usize,
    pub top5: usize,

    /// 整句候選（第一個蓋住全部拼音的候選）就是原句。
    pub sentence_hit: usize,

    /// 整句候選與原句逐字比對：對上的字數 / 總字數。
    pub chars_correct: usize,
    pub chars_total: usize,

    /// 查詢耗時之和與最大值。
    pub query_time: Duration,
    pub slowest_query: Duration,

    /// 碼錶覆蓋率；沒裝碼錶時為 `None`。
    pub coverage: Option<Coverage>,

    /// 沒命中首選的例子。
    pub misses: Vec<String>,
}

impl Report {
    pub fn evaluated(&self) -> usize {
        self.total - self.unparsable
    }
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "-".to_owned()
    } else {
        format!("{:.1}%", part as f64 * 100.0 / whole as f64)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "整句评测（冷启动，不学习，不写用户数据）")?;
        let evaluated = self.evaluated();
        writeln!(
            f,
            "句子 {:>5} 条  首选 {:>6}  前三 {:>6}  前五 {:>6}  整句候选 {:>6}  字准确率 {:>6}",
            self.total,
            percent(self.top1, evaluated),
            percent(self.top3, evaluated),
            percent(self.top5, evaluated),
            percent(self.sentence_hit, evaluated),
            percent(self.chars_correct, self.chars_total),
        )?;
        if evaluated > 0 {
            writeln!(
                f,
                "查询平均 {:.1} ms，最慢 {:.1} ms",
                self.query_time.as_secs_f64() * 1000.0 / evaluated as f64,
                self.slowest_query.as_secs_f64() * 1000.0,
            )?;
        }
        if let Some(coverage) = &self.coverage {
            writeln!(
                f,
                "码表覆盖率：词频前 {} 条 {}（{} / {}），全库 {}（{} / {}）",
                coverage.top_total,
                percent(coverage.top_hits, coverage.top_total),
                coverage.top_hits,
                coverage.top_total,
                percent(coverage.all_hits, coverage.all_total),
                coverage.all_hits,
                coverage.all_total,
            )?;
        }
        if self.unparsable > 0 {
            writeln!(f, "其中 {} 条拼音切不动", self.unparsable)?;
        }
        if self.untranscribable > 0 {
            writeln!(
                f,
                "抽出 {} 条，{} 条有字查不到读音，跳过",
                self.extracted, self.untranscribable
            )?;
        }
        if !self.misses.is_empty() {
            writeln!(f, "\n没命中首选的例子：")?;
            for miss in &self.misses {
                writeln!(f, "  {miss}")?;
            }
        }
        Ok(())
    }
}
