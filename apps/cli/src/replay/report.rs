use std::fmt;

use qingjian_core::InputSource;

use super::tally::Tally;

/// 回放報告：按來源分組的計數，加上幾條沒命中的例子。
#[derive(Debug, Default)]
pub struct Report {
    /// 詞庫詞。
    pub word: Tally,

    /// 本地整句。
    pub sentence: Tally,

    /// 英文候選。
    pub english: Tally,

    /// 快捷候選與 emoji。
    pub other: Tally,

    /// 不評的來源（雲端詞、雲端整句、原樣上屏、譯詞）各自的條數。
    pub skipped: Vec<(InputSource, usize)>,

    /// 撤銷條數。
    pub retracts: usize,

    /// 重打條數（組句內退格重打 + 上屏後刪掉重打）。
    pub retypes: usize,

    /// 會話行數、上文斷開數、直通字元數。
    pub sessions: usize,
    pub breaks: usize,
    pub passthrough_chars: usize,

    /// 雲端聯想展示了幾次、其中緊接著被接受（上屏來源是雲端詞 / 雲端整句）幾次。
    pub predictions: usize,
    pub predictions_accepted: usize,

    /// 上一條是聯想、還沒等到接下來的上屏。
    pub prediction_pending: bool,

    /// 輔碼態選詞的條數，以及其中「同樣的拼音、純拼音輸入下已排在首選」的條數。
    pub aux_total: usize,
    pub aux_top1: usize,

    /// 舊格式的空行（鍵與文本都空的原樣上屏）。
    pub empty: usize,

    /// 解析不了的行數。
    pub unparsable: usize,

    /// 沒命中首選的例子。
    pub misses: Vec<String>,
}

impl Report {
    /// 這個來源的計數板；不評的來源返回 `None`（另計到 `skipped`）。
    pub fn tally_for(&mut self, source: InputSource) -> Option<&mut Tally> {
        match source {
            InputSource::Word => Some(&mut self.word),
            InputSource::Sentence => Some(&mut self.sentence),
            InputSource::English => Some(&mut self.english),
            InputSource::Shortcut | InputSource::Emoji => Some(&mut self.other),
            InputSource::Custom
            | InputSource::Cloud
            | InputSource::CloudSentence
            | InputSource::Raw
            | InputSource::Translation => None,
        }
    }

    pub fn skip(&mut self, source: InputSource) {
        match self.skipped.iter_mut().find(|(s, _)| *s == source) {
            Some((_, count)) => *count += 1,
            None => self.skipped.push((source, 1)),
        }
    }
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "-".to_owned()
    } else {
        format!("{:.1}%", part as f64 * 100.0 / whole as f64)
    }
}

fn write_tally(f: &mut fmt::Formatter<'_>, name: &str, tally: &Tally) -> fmt::Result {
    if tally.total == 0 {
        return Ok(());
    }
    let evaluated = tally.evaluated();
    writeln!(
        f,
        "{name:<6} {:>5} 条  首选 {:>6}  前五 {:>6}  更靠后 {:>4}  不在候选 {:>4}  平均名次 {}  命中数 {} / {}",
        tally.total,
        percent(tally.top1, evaluated),
        percent(tally.top1 + tally.top5, evaluated),
        tally.found_later,
        tally.missing,
        tally
            .mean_rank()
            .map_or("-".to_owned(), |rank| format!("{rank:.2}")),
        tally.top1,
        evaluated,
    )?;
    if tally.unparsable > 0 {
        writeln!(
            f,
            "       其中 {} 条现在切不动（方案或模式键变了）",
            tally.unparsable
        )?;
    }
    if tally.corrected_then > 0 {
        writeln!(
            f,
            "       当时纠错生效 {} 条，现在仍纠 {} 条",
            tally.corrected_then, tally.corrected_now
        )?;
    }
    if let Some(((hit_pages, hit_ms), (miss_pages, miss_ms))) = tally.implicit_signal() {
        writeln!(
            f,
            "       当时首选命中 {}：平均翻页 {hit_pages:.2} / 耗时 {hit_ms:.0} ms；没命中 {}：平均翻页 {miss_pages:.2} / 耗时 {miss_ms:.0} ms",
            percent(tally.then_top1, tally.total),
            tally.total - tally.then_top1,
        )?;
    }
    if tally.rescored_then > 0 {
        let plain = tally.total - tally.rescored_then;
        let plain_top1 = tally.then_top1 - tally.rescored_then_top1;
        writeln!(
            f,
            "       当时经过神经重排 {} 条首选命中 {}，未重排 {} 条首选命中 {}",
            tally.rescored_then,
            percent(tally.rescored_then_top1, tally.rescored_then),
            plain,
            percent(plain_top1, plain),
        )?;
    }
    Ok(())
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "回放评测（内存学习，不写文件）")?;
        write_tally(f, "词", &self.word)?;
        write_tally(f, "整句", &self.sentence)?;
        write_tally(f, "英文", &self.english)?;
        write_tally(f, "其他", &self.other)?;
        if !self.skipped.is_empty() {
            let parts: Vec<String> = self
                .skipped
                .iter()
                .map(|(source, count)| format!("{source:?} {count}"))
                .collect();
            writeln!(f, "不评的来源：{}", parts.join("，"))?;
        }
        if self.retracts > 0 {
            writeln!(f, "退格撤销 {} 次", self.retracts)?;
        }
        if self.retypes > 0 {
            writeln!(f, "退格重打 {} 次", self.retypes)?;
        }
        if self.predictions > 0 {
            writeln!(
                f,
                "云端联想展示 {} 次，紧接着被接受 {} 次（{}）",
                self.predictions,
                self.predictions_accepted,
                percent(self.predictions_accepted, self.predictions)
            )?;
        }
        if self.sessions + self.breaks + self.passthrough_chars > 0 {
            writeln!(
                f,
                "会话 {} 次，上文断开 {} 次，直通字符 {} 个",
                self.sessions, self.breaks, self.passthrough_chars
            )?;
        }
        if self.aux_total > 0 {
            writeln!(
                f,
                "辅码选词 {} 条，其中同拼音纯输入首选命中 {} 条（{}）",
                self.aux_total,
                self.aux_top1,
                percent(self.aux_top1, self.aux_total)
            )?;
        }
        if self.empty > 0 {
            writeln!(f, "旧格式的空行 {} 条（不算数）", self.empty)?;
        }
        if self.unparsable > 0 {
            writeln!(f, "解析不了 {} 行", self.unparsable)?;
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
