use qingjian_dictionary::SyllablePattern;

/// 一串模式擴充套件後的結果：每個位置若干寫法，第一種是使用者敲的（代價 0），其餘是模糊音（扣 ln 2）
/// 或敲錯變體（按類別與個人敲錯表定代價）。擁有字串，借出 [`SyllablePattern`]。
#[derive(Debug, Clone, Default)]
pub struct Expanded {
    /// 每個位置的 (寫法, 代價) 與「是否完整音節」（同一位置的寫法完整性相同）。
    positions: Vec<(Vec<(String, f64)>, bool)>,

    /// 有沒有任何一個位置帶了敲的原樣以外的寫法：沒有就不必逐條命中算代價。
    alternatives: bool,
}

impl Expanded {
    pub(super) fn new(positions: impl IntoIterator<Item = (Vec<(String, f64)>, bool)>) -> Self {
        let positions: Vec<_> = positions.into_iter().collect();
        let alternatives = positions.iter().any(|(forms, _)| forms.len() > 1);
        Self {
            positions,
            alternatives,
        }
    }

    /// 給 `index` 位置加一種寫法；已有同樣寫法時只留代價低的那個。不完整的位置（簡拼、字首）不加。
    pub fn push_alternative(&mut self, index: usize, text: &str, cost: f64) {
        let Some((forms, complete)) = self.positions.get_mut(index) else {
            return;
        };
        if !*complete {
            return;
        }
        match forms.iter_mut().find(|(t, _)| t == text) {
            Some(existing) => existing.1 = existing.1.min(cost),
            None => forms.push((text.to_owned(), cost)),
        }
        self.alternatives = true;
    }

    /// 詞庫多寫法查詢要的形狀。
    pub fn positions(&self) -> Vec<Vec<SyllablePattern<'_>>> {
        self.positions
            .iter()
            .map(|(forms, complete)| {
                forms
                    .iter()
                    .map(|(text, _)| SyllablePattern {
                        text,
                        complete: *complete,
                    })
                    .collect()
            })
            .collect()
    }

    /// 有沒有敲的原樣以外的寫法。
    pub fn has_alternatives(&self) -> bool {
        self.alternatives
    }

    /// `index` 位置命中音節 `syllable` 的代價：敲的原樣 0，模糊音 / 敲錯變體按寫法記的代價；哪種寫法都對不上（不該發生）算 0。
    pub fn cost(&self, index: usize, syllable: &str) -> f64 {
        let Some((forms, complete)) = self.positions.get(index) else {
            return 0.0;
        };
        forms
            .iter()
            .find(|(text, _)| {
                SyllablePattern {
                    text,
                    complete: *complete,
                }
                .accepts(syllable)
            })
            .map_or(0.0, |(_, cost)| *cost)
    }

    /// 一條命中的總代價：各位置代價之和。沒有任何替代寫法時直接 0。
    pub fn penalty<'a>(&self, syllables: impl IntoIterator<Item = &'a str>) -> f64 {
        if !self.alternatives {
            return 0.0;
        }
        syllables
            .into_iter()
            .enumerate()
            .map(|(index, syllable)| self.cost(index, syllable))
            .sum()
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn costs_follow_the_matched_form() {
        let mut expanded = Expanded::new([
            (vec![("mei".to_owned(), 0.0)], true),
            (
                vec![("gan".to_owned(), 0.0), ("gang".to_owned(), 0.7)],
                true,
            ),
            (vec![("x".to_owned(), 0.0)], false),
        ]);
        expanded.push_alternative(1, "guan", 4.5);
        expanded.push_alternative(1, "gang", 4.5); // 已有更便宜的模糊音寫法，保留 0.7
        expanded.push_alternative(2, "xi", 4.0); // 不完整的位置不加
        assert!(expanded.has_alternatives());
        assert_eq!(expanded.penalty(["mei", "gan", "xi"]), 0.0);
        assert_eq!(expanded.penalty(["mei", "guan", "xia"]), 4.5);
        assert_eq!(expanded.penalty(["mei", "gang", "xi"]), 0.7);
        assert_eq!(expanded.positions()[1].len(), 3);
        let plain = Expanded::new([(vec![("ni".to_owned(), 0.0)], true)]);
        assert!(!plain.has_alternatives());
        assert_eq!(plain.penalty(["mi"]), 0.0);
    }
}
