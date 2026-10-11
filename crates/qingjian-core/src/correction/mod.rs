//! 拼寫糾錯：使用者敲的拼音「不像話」時（切不乾淨，或非末尾有簡拼 / 殘缺音節），
//! 試一處編輯（相鄰換位、換一個字母、多一個、少一個）能不能變成每個音節都完整的拼音。
//! 相鄰換位另放寬到「末尾音節還沒敲完」（`mignt` → `ming t…`、`migntia` → `ming tia…`）：
//! 敲反兩個鍵的人多半還在往下敲，等整個詞敲完再糾就晚了一拍。
//!
//! 這裡只產生**候選糾正**（變體 + 完整切分），挑哪一個由 Engine 用詞庫和語言模型定：
//! 糾正後至少要能湊出一個兩音節以上的詞，否則寧可不糾。學習方面：接受過的糾正按原輸入串記選擇，
//! 回車原樣上屏過的串記成「不糾」，都在 Learner 裡。
//!
//! 另一路是音節級的敲錯變體（[`typo`]）：每個完整音節一處編輯後仍合法的寫法進詞圖當帶代價的邊，
//! 由整句轉換與詞級排序按噪聲通道挑；使用者接受過的 (敲的, 要的) 音節對記進個人敲錯表（Learner），以後那條邊更便宜。各項代價與折扣上限收在 [`TypoCosts`] 裡。

mod costs;
mod edit;
mod result;
pub mod typo;

pub use costs::TypoCosts;
pub use edit::{Edit, variants};
pub use result::Correction;
pub use typo::TypoKind;

use crate::parser::{self, Segmentation};

/// 少於這麼多字母不糾：短串的一處編輯幾乎總能湊出別的合法拼音，誤糾比不糾更煩。
pub const MIN_LETTERS: usize = 4;

/// 多於這麼多字母不糾：變體數量隨長度線性漲，而且這麼長多半是整句簡拼。
pub const MAX_LETTERS: usize = 24;

/// 這段輸入是否值得試糾錯：純小寫字母、長度在範圍內。
pub fn eligible(input: &str) -> bool {
    (MIN_LETTERS..=MAX_LETTERS).contains(&input.len())
        && input.bytes().all(|b| b.is_ascii_lowercase())
}

/// 切出來的拼音「不像話」：有切不動的尾巴，或非末尾的音節裡有簡拼 / 殘缺。
/// 英文補全與拼寫糾錯都用這個判斷。
pub fn unlikely_pinyin(best: Option<&Segmentation>, tail: &str) -> bool {
    !tail.is_empty()
        || best.is_some_and(|best| {
            let count = best.syllables.len();
            best.syllables
                .iter()
                .take(count.saturating_sub(1))
                .any(|s| !s.complete)
        })
}

/// `text` 能否切成每個音節都完整的拼音；能就返回音節最少的那種切分。
pub fn complete_segmentation(text: &str) -> Option<Segmentation> {
    parser::segment(text)
        .ok()?
        .into_iter()
        .find(|s| s.incomplete_count() == 0)
}

/// `text` 能否切成「每個音節都完整，或只有末尾一個沒敲完」的拼音，按解析器的偏好取第一種：
/// `mingt` → `ming t…`，`mingtia` → `ming tia…`（不是 `ming ti a`，正常敲到這裡也是這麼切的）。
/// 末尾沒敲完時至少要有一個完整音節在前，整段都是殘缺的不算。
pub fn loose_segmentation(text: &str) -> Option<Segmentation> {
    parser::segment(text).ok()?.into_iter().find(|s| {
        s.incomplete_count() == 0
            || (s.syllables.len() >= 2 && s.incomplete_count() == 1 && s.last_is_partial())
    })
}

/// 切出來的拼音末尾是個單字母（`mingtai'n`）：可能是合法簡拼，也可能是相鄰兩鍵敲反了（`mingtain` → `mingtian`）。
pub fn trailing_single_letter(best: Option<&Segmentation>) -> bool {
    best.is_some_and(|best| {
        best.syllables.len() >= 2 && best.syllables.last().is_some_and(|s| s.text.len() == 1)
    })
}

/// `input` 的全部候選糾正：一處編輯之後能完整切分的變體，順序同 [`variants`]（換位最先）。
pub fn candidates(input: &str) -> Vec<Correction> {
    candidates_from(input, variants(input))
}

/// 只試相鄰換位的候選糾正（末尾單字母那種「像話但可疑」的輸入用，變體只有 n−1 個，每鍵都試得起）。
pub fn transposition_candidates(input: &str) -> Vec<Correction> {
    let transpositions = variants(input)
        .into_iter()
        .filter(|(edit, _)| matches!(edit, Edit::Transpose { .. }))
        .collect();
    candidates_from(input, transpositions)
}

fn candidates_from(input: &str, variants: Vec<(Edit, String)>) -> Vec<Correction> {
    if !eligible(input) {
        return Vec::new();
    }
    let found: Vec<Correction> = variants
        .into_iter()
        .filter_map(|(edit, corrected)| {
            // 相鄰換位的變體允許末尾音節沒敲完；其餘先用無分配的「能否完整切分」擋掉絕大多數
            let segmentation = if matches!(edit, Edit::Transpose { .. }) {
                loose_segmentation(&corrected)?
            } else if parser::is_fully_segmentable(&corrected) {
                complete_segmentation(&corrected)?
            } else {
                return None;
            };
            Some(Correction {
                original: input.to_owned(),
                corrected,
                edit,
                segmentation,
            })
        })
        .collect();
    // 湊得出「每個音節都完整」的變體時只在這些裡挑，末尾沒敲完的不參與：殘尾按字首能匹配到高頻詞，
    // 得分往往壓過整段完整的糾正（`keyyi` 刪一個 y 是 可以，`ke yi y…` 卻出 可以有；`weti` 的 `wei t…` 會搶出 委託）。
    // 末尾是落單單字母的完整變體不算數（`migntia` 換一個字母能湊出 `mian ti a`，那正是「可能敲反了」的可疑切法），
    // 只有一個音節的也不算數（`zehg` 把 h 換成 n 是完整的 `zeng`，但四個字母以上只拼出一個音節多半是還沒敲完），
    // 改在剛敲的最後一個鍵上的也不算數（`zehg` 把 g 換成 a 是完整的 `ze ha`，可 g 是剛敲下去的，
    // 和「刪掉剛敲的最後一個字母不算糾正」一個道理）；這些情況下末尾沒敲完的 `zhe g…` 仍參與。
    let settled = found.iter().any(|c| {
        c.segmentation.incomplete_count() == 0
            && c.segmentation.syllables.len() >= 2
            && !trailing_single_letter(Some(&c.segmentation))
            && !c.edit.touches_last_letter(c.corrected.len())
    });
    if settled {
        found
            .into_iter()
            .filter(|c| c.segmentation.incomplete_count() == 0)
            .collect()
    } else {
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eligibility_bounds() {
        assert!(!eligible("nih"));
        assert!(eligible("niho"));
        assert!(!eligible("ni'hao"));
        assert!(!eligible(&"a".repeat(MAX_LETTERS + 1)));
    }

    #[test]
    fn unlikely_when_tail_or_inner_abbreviation() {
        let (segs, tail) = (parser::segment("nihao").unwrap(), "");
        assert!(!unlikely_pinyin(segs.first(), tail));
        assert!(unlikely_pinyin(segs.first(), "v"));
        let segs = parser::segment("nihooma").unwrap();
        assert!(unlikely_pinyin(segs.first(), ""));
        // 末尾沒打完不算
        let segs = parser::segment("nihaom").unwrap();
        assert!(!unlikely_pinyin(segs.first(), ""));
    }

    #[test]
    fn personal_discount_is_capped() {
        let costs = TypoCosts::default();
        assert_eq!(costs.typo_cost(TypoKind::Transpose, 0), 5.0);
        assert!(costs.typo_cost(TypoKind::Transpose, 2) < 4.0);
        assert!(costs.typo_cost(TypoKind::Missing, 1000) >= 2.5 - 1e-9);
    }

    #[test]
    fn finds_full_syllable_variants() {
        let found = candidates("nihooma");
        let corrected: Vec<&str> = found.iter().map(|c| c.corrected.as_str()).collect();
        assert!(corrected.contains(&"nihaoma"));
        assert!(corrected.contains(&"nihouma"));
        assert!(found.iter().all(|c| c.segmentation.incomplete_count() == 0));
        // 換位也在其中（哪個勝出由 Engine 按詞庫與語言模型定）
        let found = candidates("nihoama");
        assert!(found.iter().any(
            |c| c.corrected == "nihaoma" && matches!(c.edit, Edit::Transpose { index: 3, .. })
        ));
    }

    #[test]
    fn transposition_may_leave_the_last_syllable_unfinished() {
        // 敲到一半：mignt → ming t…、migntia → ming tia…
        let found = candidates("mignt");
        let unfinished = found
            .iter()
            .find(|c| c.corrected == "mingt")
            .expect("mingt");
        assert!(matches!(unfinished.edit, Edit::Transpose { index: 2, .. }));
        assert_eq!(unfinished.segmentation.joined("'"), "ming't");
        assert!(unfinished.segmentation.last_is_partial());
        let found = candidates("migntia");
        let unfinished = found
            .iter()
            .find(|c| c.corrected == "mingtia")
            .expect("mingtia");
        assert_eq!(unfinished.segmentation.joined("'"), "ming'tia");
        // 替換 / 多敲 / 少敲仍要求每個音節都完整：migna 的替換變體 mingn… 這類不收
        assert!(
            candidates("mignt")
                .iter()
                .filter(|c| !matches!(c.edit, Edit::Transpose { .. }))
                .all(|c| c.segmentation.incomplete_count() == 0)
        );
        // 整段都殘缺的不算
        assert!(loose_segmentation("zh").is_none());
        assert!(loose_segmentation("mingt").is_some());
    }

    #[test]
    fn unfinished_variants_yield_to_fully_segmentable_ones() {
        // keyyi 刪一個 y 就是完整的 ke yi：換位出來的 ke yi y… 不參與
        let found = candidates("keyyi");
        assert!(found.iter().any(|c| c.corrected == "keyi"));
        assert!(found.iter().all(|c| c.segmentation.incomplete_count() == 0));
        // weti 補一個 n 是 wen ti；yiwne 換位是 yi wen：同樣沒有殘尾變體
        for input in ["weti", "yiwne"] {
            assert!(
                candidates(input)
                    .iter()
                    .all(|c| c.segmentation.incomplete_count() == 0),
                "{input}"
            );
        }
        // zehg 換掉 h 是單音節的 zeng、換掉剛敲的 g 是 ze ha：都不算數，zhe g… 仍在
        assert!(
            candidates("zehg")
                .iter()
                .any(|c| c.corrected == "zheg" && c.segmentation.last_is_partial())
        );
        // migntia 換一個字母能湊出 mian ti a，但末尾落單的 a 不算「完整」：ming tia… 仍在
        assert!(
            candidates("migntia")
                .iter()
                .any(|c| c.corrected == "mingtia" && c.segmentation.last_is_partial())
        );
    }
}
