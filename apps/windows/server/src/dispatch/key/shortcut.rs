//! 組句中「修飾鍵 + 數字」的快捷鍵：上屏譯詞、刪候選。與 macOS 殼對齊。

use qingjian_core::{Candidate, CandidateList};
use qingjian_platform::protocol::KeyModifiers;

use super::Effect;
use crate::dispatch::Router;

impl Router {
    /// 配到哪組就上屏第一 / 第二個譯詞或刪候選；哪組都不是返回 `None`，按普通鍵處理。
    pub(super) fn apply_digit_shortcut(
        &mut self,
        digit: usize,
        chord: KeyModifiers,
    ) -> Option<Effect> {
        if chord == KeyModifiers::default() {
            return None;
        }
        let (first, second) = self.config.translation_keys;
        let effect = if chord == first {
            self.commit_translation_on_page(digit, 0)
        } else if chord == second {
            self.commit_translation_on_page(digit, 1)
        } else if chord == self.config.delete_keys {
            self.forget_on_page(digit)
        } else {
            return None;
        };
        Some(effect)
    }

    /// 上屏第 `digit` 個候選的第 `sense` 條譯文；沒有那條譯文就吞掉按鍵不動。
    fn commit_translation_on_page(&mut self, digit: usize, sense: usize) -> Effect {
        let Some(candidate) = self.annotated_candidate_on_page(digit) else {
            tracing::debug!(digit, "这一格没有候选");
            return Effect::Navigated;
        };
        match self.engine.commit_translation(&candidate, sense) {
            Some(text) => Effect::Changed(Some(text)),
            None => {
                tracing::debug!(digit, sense, "这个候选没有这条译文");
                Effect::Navigated
            }
        }
    }

    /// 刪第 `digit` 個候選（使用者詞整刪、詞庫詞清學習記錄），提示隨下一幀下發。
    fn forget_on_page(&mut self, digit: usize) -> Effect {
        let Some(candidate) = self.candidate_on_page(digit) else {
            tracing::debug!(digit, "这一格没有候选，没什么可删");
            return Effect::Navigated;
        };
        let forgotten = self.engine.forget(&candidate);
        let text = &candidate.text;
        let message = if forgotten.user_word {
            format!("已删除用户词「{text}」")
        } else if forgotten.learning {
            format!("已忘掉对「{text}」的学习记录")
        } else {
            format!("「{text}」是词库里的词，也没有学习记录，没什么可删")
        };
        tracing::info!(%message);
        self.notice = Some(message);
        Effect::Changed(None)
    }

    /// 當前頁第 `digit` 個候選（1 起）。
    fn candidate_on_page(&self, digit: usize) -> Option<Candidate> {
        let page_size = self.config.page_size;
        let page = self.highlight / page_size;
        let index = page * page_size + digit.checked_sub(1)?;
        self.layout_candidate(index)
    }

    /// 同上，補上譯文（佈局裡存的是查詢原樣，譯文畫頁時才補）。
    pub(super) fn annotated_candidate_on_page(&self, digit: usize) -> Option<Candidate> {
        let candidate = self.candidate_on_page(digit)?;
        let mut list = CandidateList {
            items: vec![candidate],
        };
        self.engine.annotate(&mut list);
        list.items.pop()
    }
}
