use crate::candidate::CandidateList;
use crate::correction::Correction;
use crate::parser::Segmentation;

/// 最優切分的音節用 `'` 連線，再接未切分尾部。
pub(crate) fn join_marked(segmentations: &[Segmentation], tail: &str) -> String {
    let mut text = segmentations
        .first()
        .map(|s| s.joined("'"))
        .unwrap_or_default();
    if !tail.is_empty() {
        if !text.is_empty() {
            text.push('\'');
        }
        text.push_str(tail);
    }
    text
}

/// 與 [`join_marked`] 相同的分段，但用原樣大小寫的輸入（`Cpan`）：切分是按小寫算的，
/// 大小寫隻影響顯示，逐段按同樣的位元組長度取回原樣文本。
pub(crate) fn join_marked_typed(typed: &str, segmentations: &[Segmentation], tail: &str) -> String {
    let mut text = String::new();
    let mut offset = 0;
    if let Some(first) = segmentations.first() {
        for (index, syllable) in first.syllables.iter().enumerate() {
            if index > 0 {
                text.push('\'');
            }
            let end = (offset + syllable.text.len()).min(typed.len());
            text.push_str(&typed[offset..end]);
            offset = end;
        }
    }
    if !tail.is_empty() {
        if !text.is_empty() {
            text.push('\'');
        }
        text.push_str(&typed[offset.min(typed.len())..]);
    }
    text
}

use crate::engine::timings::Timings;
use crate::engine::{AuxSegment, MarkedKind, MarkedSegment};

/// 不帶譯文的候選查詢結果。
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// 參與候選生成的所有切分，索引 0 為首選切分。
    pub segmentations: Vec<Segmentation>,

    /// 排好序的候選，`translation` 均為 `None`。
    pub candidates: CandidateList,

    /// 輸入末尾無法切分的字母（如 `kaifv` 的 `v`），不參與本次候選，留給後續輸入。
    pub tail: String,

    /// 查詢時緩衝區裡的原始文本。
    pub text: String,

    /// 查詢時的游標位置（`text` 的位元組下標）。
    pub cursor: usize,

    /// 游標停在中間時，作用域之後剩下的拼音的顯示形式（已按音節用 `'` 連好）；候選不管它，只畫出來。
    pub rest: String,

    /// 各階段耗時。
    pub timings: Timings,

    /// 生效的拼寫糾正：`segmentations` 與候選都來自糾正後的拼音，`text` 仍是使用者敲的。
    pub correction: Option<Correction>,

    /// 雙拼 / 注音開著：`segmentations` 是解出來的拼音，`text` 是敲的鍵，兩者長度對不上。
    pub decoded_keys: bool,

    /// 雙拼模式下是否在 preedit（應用輸入框內 marked text）保留原始輸入按鍵。
    pub shuangpin_raw_preedit: bool,

    /// 開啟雙拼或注音時的顯示字串（如 "ㄅㄨˋ"）。如果有此值，preedit 就優先顯示它，而不是拼音。
    pub typed_display: Option<String>,

    /// 輔碼態：觸發鍵與碼段（碼段可為空——剛觸發）。`Some` 時 preedit 在拼音段之後多出
    /// [觸發鍵 `Typed`][碼段 `AuxCode`] 兩段，候選也已經按碼段篩過。
    pub aux: Option<AuxSegment>,
}

impl Query {
    /// 無法解析為拼音但精確匹配自定義短語時，保留原始輸入和游標。
    pub(super) fn custom_only(
        text: &str,
        cursor: usize,
        decoded_keys: bool,
        shuangpin_raw_preedit: bool,
        scope: &str,
        rest: String,
    ) -> Self {
        Self {
            text: text.to_owned(),
            cursor,
            decoded_keys,
            shuangpin_raw_preedit,
            tail: scope.to_owned(),
            rest,
            ..Self::default()
        }
    }

    /// 給 marked text（應用輸入框未上屏文本）用的顯示形式：最優切分的音節用 `'` 連線，再接未切分尾部，
    /// 輔碼態接上觸發鍵與碼段，游標後的剩餘拼音跟在最後。
    /// `kaifa` → `kai'fa`，`kf` → `k'f`，`ni|hao` → `ni'hao`，`nihao;rb` → `ni'hao;rb`。
    /// 雙拼模式且 `shuangpin_raw_preedit` 開啟時，返回原始按鍵（如 `kdfa`）。
    pub fn marked_text(&self) -> String {
        if self.shuangpin_raw_preedit {
            return self.text.clone();
        }
        self.marked_segments()
            .iter()
            .map(|s| s.text.as_str())
            .collect()
    }

    /// [`Self::marked_text`] 的分段形式：敲的拼音一段（`Typed`），游標後剩下的拼音連同前面的 `'` 一段（`Rest`）。
    /// 殼按段畫樣式；[`Self::segments_cursor`] 的位置按各段拼接後的字元數算。
    pub fn marked_segments(&self) -> Vec<MarkedSegment> {
        let mut segments = match &self.correction {
            Some(correction) => correction.marked_segments(),
            None => Vec::with_capacity(2),
        };
        if self.correction.is_none() {
            let typed = if let Some(display) = &self.typed_display {
                display.clone()
            } else {
                join_marked(&self.segmentations, &self.tail)
            };
            if !typed.is_empty() {
                segments.push(MarkedSegment::new(typed, MarkedKind::Typed));
            }
        }
        if let Some(aux) = &self.aux {
            // 觸發鍵按 Typed 畫（不突出），碼段是新段型別；碼段為空時只多這一段觸發鍵
            segments.push(MarkedSegment::new(
                aux.trigger.to_string(),
                MarkedKind::Typed,
            ));
            if !aux.code.is_empty() {
                segments.push(MarkedSegment::new(aux.code.clone(), MarkedKind::AuxCode));
            }
        }
        if !self.rest.is_empty() {
            let rest = if segments.is_empty() {
                self.rest.clone()
            } else {
                format!("'{}", self.rest)
            };
            segments.push(MarkedSegment::new(rest, MarkedKind::Rest));
        }
        segments
    }

    /// 游標在 [`Self::marked_segments`] 拼接文本里的字元下標（候選視窗頂部拼音行使用）。
    pub fn segments_cursor(&self) -> usize {
        // 糾錯生效、解碼時顯示串與敲的不一樣長，作用域又總在游標前：游標就在敲的部分末尾
        // （游標在開頭時作用域是整段，游標仍在開頭）
        if self.decoded_keys && self.cursor == 0 {
            return 0;
        }
        // 輔碼態游標也總在末尾：觸發鍵與碼段都拼在最後
        if self.correction.is_some() || self.decoded_keys || self.aux.is_some() {
            return self
                .marked_segments()
                .iter()
                .filter(|s| s.kind != MarkedKind::Rest)
                .map(|s| s.text.chars().count())
                .sum();
        }
        let letters_before = self.text[..self.cursor.min(self.text.len())]
            .chars()
            .filter(|c| *c != '\'')
            .count();
        let after_apostrophe = self.text[..self.cursor.min(self.text.len())].ends_with('\'');
        let segments_text: String = self
            .marked_segments()
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        let chars: Vec<char> = segments_text.chars().collect();
        let mut seen = 0;
        let mut position = 0;
        while position < chars.len() && seen < letters_before {
            if chars[position] != '\'' {
                seen += 1;
            }
            position += 1;
        }
        if after_apostrophe && chars.get(position) == Some(&'\'') {
            position += 1;
        }
        position
    }

    /// 游標在 [`Self::marked_text`] 裡的字元下標（給平臺層傳給宿主應用輸入框用的，所以按字元算，不是位元組）。
    pub fn marked_cursor(&self) -> usize {
        if self.shuangpin_raw_preedit {
            return self.text[..self.cursor.min(self.text.len())]
                .chars()
                .count();
        }
        self.segments_cursor()
    }
}
