//! 詞級查詢：每個位置展開成多種寫法，再按這些寫法查主詞庫與使用者詞。

use super::*;

impl Engine {
    /// 每個位置的寫法：敲的原樣、模糊音，再加音節級敲錯變體（`correction::typo`）當帶代價的邊，
    /// 代價按類別定、按個人敲錯表打折。太短的輸入（不到 [`correction::MIN_LETTERS`]）、雙拼、非末尾帶簡拼的切分不加敲錯變體：
    /// 短串一處編輯幾乎總能湊出別的詞，雙拼敲錯一鍵換掉的是整個聲母 / 韻母。不完整的位置（簡拼、字首）本來就按字首查，不加。
    pub(in crate::engine) fn expand_positions(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
        typos: bool,
    ) -> Expanded {
        let mut expanded = self.fuzzy.expand(patterns);
        if !typos {
            return expanded;
        }
        let letters: usize = patterns.iter().map(|p| p.text.len()).sum();
        // 非末尾有簡拼 / 殘缺音節的切分（`kai f a`）本來就不是使用者敲的原話，不在它上面再猜敲錯
        let inner_abbreviated = patterns
            .iter()
            .take(patterns.len().saturating_sub(1))
            .any(|p| !p.complete);
        if self.shuangpin.is_some() || letters < correction::MIN_LETTERS || inner_abbreviated {
            return expanded;
        }
        for (index, pattern) in patterns.iter().enumerate() {
            if !pattern.complete {
                continue;
            }
            for (text, kind) in typo::variants(pattern.text) {
                let accepted = self.learner.typo_count(pattern.text, text);
                expanded.push_alternative(index, text, self.typo_costs.typo_cost(*kind, accepted));
            }
        }
        expanded
    }

    /// 本地整句轉換把最優切分轉成的漢字，給雲端當參考（問字模式裡就是問題的漢字形式）；轉不出或有佔位音節為空。
    pub(in crate::engine) fn local_guess(&self, segmentations: &[Segmentation]) -> String {
        segmentations
            .first()
            .and_then(|best| self.convert_sentence(&best.patterns(), true))
            .filter(|conversion| !conversion.has_placeholder())
            .map(|conversion| conversion.text)
            .unwrap_or_default()
    }

    /// 主詞庫與使用者詞一起查（每個位置多種寫法）。使用者詞是使用者自己選過的（雲聯想接受的詞等），排序上靠 weight 自然靠前。
    pub(in crate::engine) fn lookup_all(
        &self,
        positions: &[Vec<qingjian_dictionary::SyllablePattern<'_>>],
    ) -> Vec<Match<'_>> {
        let mut hits = self.dictionary.lookup_pattern_alt(positions);
        for dictionary in self.all_dictionaries().into_iter().skip(1) {
            hits.extend(dictionary.lookup_pattern_alt(positions));
        }
        hits
    }

    /// 只要音節數正好等於位置數的詞，主詞庫與使用者詞一起查。
    pub(in crate::engine) fn lookup_exact_all(
        &self,
        positions: &[Vec<qingjian_dictionary::SyllablePattern<'_>>],
    ) -> Vec<Match<'_>> {
        let mut hits = self.dictionary.lookup_exact_alt(positions);
        for dictionary in self.all_dictionaries().into_iter().skip(1) {
            hits.extend(dictionary.lookup_exact_alt(positions));
        }
        hits
    }
}
