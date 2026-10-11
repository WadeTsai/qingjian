//! 碼錶覆蓋率：詞庫裡的詞有多少能查到碼。
//!
//! 與匯入統計同源：都用碼錶的 `AuxCodeLookup`，兩段口徑分別是「詞頻前 10,000」與「全庫」。

use qingjian_core::Engine;

/// 覆蓋率只看詞頻最高的這麼多條（預設口徑）。
pub const COVERAGE_TOP: usize = 10_000;

/// 兩段覆蓋率的分子分母。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    /// 詞頻前 `COVERAGE_TOP` 條裡能查到碼的條數 / 總數。
    pub top_hits: usize,
    pub top_total: usize,

    /// 全庫能查到碼的條數 / 總數。
    pub all_hits: usize,
    pub all_total: usize,
}

/// 算一遍覆蓋率；沒裝碼錶時返回 `None`（這條尺子只在 `--aux-table` 下才有意義）。
pub fn measure(engine: &Engine) -> Option<Coverage> {
    let tables = engine.aux_codes();
    if tables.is_empty() {
        return None;
    }
    let covers = |text: &str| {
        tables
            .iter()
            .any(|table| table.code_with_prefix(text, "").is_some())
    };
    let mut words: Vec<(&str, u32)> = engine
        .dictionary()
        .entries()
        .map(|hit| (hit.text, hit.frequency))
        .collect();
    let all_total = words.len();
    let all_hits = words.iter().filter(|(text, _)| covers(text)).count();
    words.sort_by_key(|(_, frequency)| std::cmp::Reverse(*frequency));
    words.truncate(COVERAGE_TOP);
    let top_total = words.len();
    let top_hits = words.iter().filter(|(text, _)| covers(text)).count();
    Some(Coverage {
        top_hits,
        top_total,
        all_hits,
        all_total,
    })
}
