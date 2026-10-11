//! 整段拼寫糾錯：找一處編輯的糾正並按個人敲錯表打折。

use super::*;

impl Engine {
    /// 這段作用域生效的拼寫糾正（帶快取）：拼音不像話、使用者沒對它回車原樣上屏過、
    /// 且一處編輯後能湊出至少一個兩音節詞時，取整句轉換得分最高的那個糾正。
    pub(super) fn active_correction(&self, scope: &str) -> Option<Correction> {
        // 雙拼敲錯一個鍵換掉的是整個聲母 / 韻母，全拼那套「一處編輯」的糾錯模型不適用
        if self.shuangpin.is_some() {
            return None;
        }
        // 形碼敲的是字根編碼，不是拼音；混輸下四碼以內可能是編碼，超過四碼才只剩拼音
        if self.code.is_some() && (!self.phonetic || scope.len() <= MAX_CODE_LENGTH) {
            return None;
        }
        if let Some((cached_scope, cached)) = self.correction_cache.borrow().as_ref()
            && cached_scope == scope
        {
            return cached.clone();
        }
        let found = self.find_correction(scope);
        *self.correction_cache.borrow_mut() = Some((scope.to_owned(), found.clone()));
        found
    }

    pub(super) fn find_correction(&self, scope: &str) -> Option<Correction> {
        if !correction::eligible(scope) || self.learner.raw_count(scope) > 0 {
            return None;
        }
        // 整段就是個英文詞（hello）：使用者多半在打英文，別把它「糾」成 喝了哦
        if self
            .english
            .as_ref()
            .is_some_and(|english| english.get(scope).is_some())
        {
            return None;
        }
        let (segmentations, tail) = segment_longest_prefix(scope).ok()?;
        // 不像話的拼音試全部一處編輯；末尾單字母的只試相鄰換位（`mingtain` → `mingtian`），其餘合法拼音不碰
        let candidates = if correction::unlikely_pinyin(segmentations.first(), tail) {
            correction::candidates(scope)
        } else if correction::trailing_single_letter(segmentations.first()) {
            correction::transposition_candidates(scope)
        } else {
            return None;
        };
        // 噪聲通道：原串按原樣能轉出的整句得分 vs 糾正後的整句得分扣掉一次編輯的代價，後者高才糾。
        // 原串切不乾淨（有尾巴）就沒有原樣得分，任何能轉出整句的糾正都勝出
        let raw_score = if tail.is_empty() {
            segmentations
                .first()
                .and_then(|best| self.convert_sentence(&best.patterns(), true))
                .filter(|conversion| !conversion.has_placeholder())
                .map(|conversion| conversion.score)
        } else {
            None
        };
        // 每個糾正扣一次編輯的代價，接受過同樣的 (敲的, 要的) 音節對越多次扣得越少（個人敲錯表）。
        // 相鄰換位的折扣只在糾正之間比較時用（`mignti` 換位成 `mingti` 勝過換字母成 `mianti`），
        // 與原樣比時不打這個折：要不要糾的門檻不因編輯型別而降（`zhongwne` 原樣說得通就不糾成 中文）
        let mut best: Option<(f64, f64, Correction)> = None;
        for candidate in candidates {
            // 「刪掉剛敲的最後一個字母」不算糾正：使用者可能還沒敲完，尾巴留著等下一鍵
            if matches!(candidate.edit, correction::Edit::Delete { index, .. } if index + 1 == scope.len())
            {
                continue;
            }
            // 糾正後的拼音上不再猜第二處敲錯：變體本來就是一處編輯之外的讀法，再疊一層既慢又幾乎不會贏
            let Some(conversion) = self.convert_sentence(&candidate.segmentation.patterns(), false)
            else {
                continue;
            };
            if conversion.has_placeholder() {
                continue;
            }
            let accepted = candidate
                .typo_pair(candidate.corrected.len())
                .map_or(0, |(typed, intended)| {
                    self.learner.typo_count(&typed, &intended)
                });
            let transpose = matches!(candidate.edit, correction::Edit::Transpose { .. });
            let score = conversion.score - self.typo_costs.correction_cost(transpose, accepted);
            let undiscounted = conversion.score - self.typo_costs.correction_cost(false, accepted);
            if best.as_ref().is_none_or(|(best, _, _)| score > *best) {
                best = Some((score, undiscounted, candidate));
            }
        }
        let (_, score, found) = best?;
        if raw_score.is_some_and(|raw| score <= raw) {
            tracing::debug!(
                original = %found.original,
                corrected = %found.corrected,
                score,
                raw = raw_score,
                "原样已经说得通，不纠"
            );
            return None;
        }
        tracing::debug!(original = %found.original, corrected = %found.corrected, score, raw = raw_score, "拼写纠正");
        Some(found)
    }
}
