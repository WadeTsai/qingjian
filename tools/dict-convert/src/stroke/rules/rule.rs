//! 大陸筆順覆蓋規則：模式、替換與說明。

use std::collections::HashSet;

use crate::stroke::rules::anchor::Anchor;

/// 一條部件重寫規則：CNS（台灣序）裡的 `from` 模式就是大陸的 `to`。
pub(super) struct Rule {
    /// 部件名（覆蓋表裡的標識，報錯與日誌用）。
    pub(super) name: String,

    /// CNS 序列裡的部件筆畫模式。
    pub(super) from: String,

    /// 大陸規範裡的部件筆畫模式。
    pub(super) to: String,

    /// 匹配位置。
    pub(super) anchor: Anchor,

    /// 例外字：這些字裡命中的模式不是該部件，規則不適用。
    pub(super) skips: HashSet<char>,
}
