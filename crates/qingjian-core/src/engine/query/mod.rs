//! 候選生成：按輸入模式分派查詢。各模式的實現在兄弟檔案裡，共用的候選構造留在這裡。

use super::*;

mod code;
mod converting;
mod english_tail;
mod generating;
mod lookup;
mod modes;
mod phonetic;
mod result;
mod snapshot;

pub(crate) use english_tail::EnglishTail;
pub use result::Query;
pub(super) use result::join_marked;
pub(super) use result::join_marked_typed;
pub(super) use snapshot::QuerySnapshot;

impl Engine {
    /// 解析當前緩衝區並生成排好序的候選。**不帶譯文**，譯文由 [`Self::annotate`] 補。
    ///
    /// 游標停在拼音中間時只按游標前的那段算候選（`ni|hao` 出 你），游標後的拼音留著，
    /// 上屏之後接著組句；見 [`Composition::scope`]。
    pub fn query(&self) -> Result<Query, ParseError> {
        self.last_rescored.set(false);
        let mut query = match self.query_inner() {
            Ok(query) => query,
            Err(error) => {
                if !self
                    .custom_phrases
                    .iter()
                    .any(|p| p.enabled && p.code == self.composition.scope())
                {
                    return Err(error);
                }
                Query::custom_only(
                    &self.composition.typed_text(),
                    self.composition.cursor(),
                    self.shuangpin.is_some() || self.zhuyin.is_some(),
                    self.shuangpin.is_some() && self.shuangpin_raw_preedit,
                    self.composition.scope(),
                    self.marked_rest(self.composition.rest()),
                )
            }
        };
        query.aux = self.aux_segment();
        // 輔碼態只出命中碼的詞：自定義短語沒有碼，不出
        if self.aux_filter().is_none() {
            self.insert_custom_phrases(&mut query.candidates.items);
        }
        // 給輸入日誌留個摘要：上屏時才知道選了什麼，這裡才知道看到了什麼
        let pinyin = match &query.correction {
            Some(correction) => correction.segmentation.joined("'"),
            None => join_marked(&query.segmentations, &query.tail),
        };
        *self.last_query.borrow_mut() = Some(QuerySnapshot {
            scope: self.composition.scope().to_owned(),
            pinyin,
            corrected: query.correction.is_some(),
            candidates: query
                .candidates
                .items
                .iter()
                .take(QuerySnapshot::MAX_CANDIDATES)
                .map(|c| c.text.clone())
                .collect(),
            rescored: self.last_rescored.get(),
        });

        if self.traditional
            && let Some(opencc) = &self.opencc
        {
            // 除英文詞與 emoji 外都轉：模型直接生成的整句、形碼碼錶的字詞、自定義短語與快捷候選（中文數字）
            // 也是簡體出來的，漏了哪一種使用者就會在繁體模式下看到簡體。
            for candidate in &mut query.candidates.items {
                if !matches!(
                    candidate.kind,
                    CandidateKind::English | CandidateKind::Emoji
                ) {
                    let traditional_text = opencc.convert(&candidate.text);
                    self.traditional_map
                        .borrow_mut()
                        .insert(traditional_text.clone(), candidate.text.clone());
                    candidate.text = traditional_text;
                }
            }
        }

        Ok(query)
    }

    pub(super) fn query_inner(&self) -> Result<Query, ParseError> {
        let start = Instant::now();
        let keys = self.composition.scope();
        let rest = self.marked_rest(self.composition.rest());
        if self.english_mode {
            return Ok(self.query_english(keys, rest, start));
        }
        if self.modes().is_expression(keys, self.zhuyin) {
            return Ok(self.query_expression(keys, rest, start));
        }
        if self.modes().is_question(keys, self.zhuyin) {
            return Ok(self.query_question(keys, rest, start));
        }
        if is_raw(keys, self.modes(), self.shuangpin, self.zhuyin) {
            return Ok(self.query_raw(keys, rest, start));
        }
        // 形碼與拼音是兩條平行的管線，在進切分之前分岔。放在這裡是為了讓 `?` 問字與
        // `-` 直輸段仍然先分派出去：形碼下 `v` / `u` / `i` 是字根鍵，模式鍵已由 `modes()` 讓位。
        match self.code.is_some() {
            // 只用形碼：拼音側整個不走（`[general] scheme = "none"`）
            true if !self.phonetic => Ok(self.query_code(keys, rest, start)),
            // 混輸：兩邊都出候選
            true => self.query_mixed(keys, rest, start),
            false => self.query_phonetic(keys, rest, start),
        }
    }
}

/// 詞庫命中的中文候選。`aux_code` 是給殼顯示的輔碼：篩碼時是命中當前碼段的那條，沒在篩碼
/// （純拼音態、輔碼態空碼段）時是詞的首條碼；沒裝碼錶或這個詞沒有碼時是 `None`。
pub(super) fn chinese_candidate(item: &Scored<'_>, aux_code: Option<&str>) -> Candidate {
    Candidate {
        text: item.hit.text.to_owned(),
        kind: CandidateKind::Chinese,
        syllables: item.hit.syllables().map(str::to_owned).collect(),
        reading: None,
        translation: None,
        aux_code: aux_code.map(str::to_owned),
    }
}

/// 排在開頭的英文候選有幾條（整段是英文詞、不像拼音帶出的英文補全）：整句插在它們後面。
pub(super) fn leading_english(items: &[Candidate]) -> usize {
    items
        .iter()
        .take_while(|c| c.kind == CandidateKind::English)
        .count()
}
