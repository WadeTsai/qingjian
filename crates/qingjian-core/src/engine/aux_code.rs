//! 輔碼態：觸發鍵的判定、碼段的增刪、碼錶查詢。碼段獨立於拼音緩衝區（見 [`Engine`] 的 `aux_code` 欄位）。

use super::{AuxSegment, Engine};
use crate::parser;

/// 觸發鍵校驗：單字元、ASCII 可列印、不是字母數字、不是 `'`（拼音隔音符）、不是當前翻頁鍵。
/// 翻頁鍵按當前配置動態排除（`[ ]`、`,.` 只有真配成 `[general] page_keys` 才拒）。設定頁與配置載入共用。
pub fn is_valid_aux_code_key(key: char, page_keys: (char, char)) -> bool {
    key.is_ascii_graphic()
        && !key.is_ascii_alphanumeric()
        && key != '\''
        && key != page_keys.0
        && key != page_keys.1
}

impl Engine {
    /// 敲下 `key` 是不是要進輔碼態。五條邊界：
    ///
    /// - 輔碼開著且有可用碼錶（`[aux_code] enabled` 預設關；沒裝表或表全被關掉都不觸發，`;` 保持原生行為）；
    /// - 配的觸發鍵，且當前不在輔碼態（已經在裡面時再敲是冪等的，不算觸發）；
    /// - 這個鍵沒被鍵盤方案吃掉：微軟 / 搜狗雙拼裡能當韻母的 `;` 優先當韻母（`x;` = xing）；
    /// - 游標在拼音段末尾（游標停在中間時只按游標前那段算候選，不是完整的輸入）；
    /// - 作用域能完整切分成音節（簡拼、殘缺音節、英文直輸段、表示式 / 問字模式都不觸發）。
    pub fn aux_trigger(&self, key: char) -> bool {
        self.aux_enabled
            && !self.aux_codes.is_empty()
            && key == self.aux_code_key
            && self.aux_code.is_none()
            && !self.english_mode
            && !(key == ';' && self.takes_semicolon())
            && self.composition.cursor() >= self.composition.text().len()
            && self.aux_scope_ready()
    }

    /// 進輔碼態：碼段清空（候選先不過濾），preedit 多出觸發鍵那一段。
    /// 觸發鍵不進緩衝區，殼在 `;` 特判旁調這裡。
    pub fn enter_aux(&mut self) {
        // 已經在輔碼態裡再調一次沒有效果：碼段原樣留著
        if self.aux_code.is_none() {
            self.aux_code = Some(String::new());
        }
    }

    /// 碼段當前內容；不在輔碼態時是空串。
    pub fn aux_code(&self) -> &str {
        self.aux_code.as_deref().unwrap_or_default()
    }

    /// 是否在輔碼態。碼段為空也算：剛敲下觸發鍵、候選還沒開始篩，或刪空碼段停在輔碼態（開關開著）。
    pub fn in_aux(&self) -> bool {
        self.aux_code.is_some()
    }

    /// 當前的觸發鍵。
    pub fn aux_code_key(&self) -> char {
        self.aux_code_key
    }

    /// 碼段追加一個字母。非 `a-z` 返回 `false`，殼按原語義處理這個鍵（翻頁鍵、標點）。
    pub fn push_aux_code(&mut self, c: char) -> bool {
        if !c.is_ascii_lowercase() {
            return false;
        }
        match &mut self.aux_code {
            Some(code) => {
                code.push(c);
                true
            }
            None => false,
        }
    }

    /// Esc：清碼段回拼音態，拼音與候選保持。
    pub fn clear_aux(&mut self) {
        self.aux_code = None;
    }

    /// 正在篩的碼段：輔碼態且碼段非空時才有。碼段空（剛觸發或刪空停住）不過濾，候選不變。
    pub(super) fn aux_filter(&self) -> Option<&str> {
        self.aux_code.as_deref().filter(|code| !code.is_empty())
    }

    /// 碼錶插樁：`word` 有沒有以 `prefix` 開頭的碼，有就返回命中的那條（多張表按注入順序取第一條）。
    pub(super) fn matching_code<'a>(&'a self, word: &str, prefix: &str) -> Option<&'a str> {
        self.aux_codes
            .iter()
            .find_map(|table| table.code_with_prefix(word, prefix))
    }

    /// preedit 裡的輔碼兩段；不在輔碼態時為 `None`。
    pub(super) fn aux_segment(&self) -> Option<AuxSegment> {
        self.aux_code
            .as_ref()
            .map(|code| AuxSegment::new(self.aux_code_key, code.clone()))
    }

    /// 作用域能不能完整切分成音節：雙拼 / 注音按解碼結果，全拼按 [`parser::is_fully_segmentable`]。
    fn aux_scope_ready(&self) -> bool {
        let scope = self.composition.scope();
        if scope.is_empty() || self.expression_mode() || self.question_mode() || self.raw_mode() {
            return false;
        }
        match self.decode(scope) {
            Some(decoded) => decoded.is_complete(),
            None => parser::is_fully_segmentable(scope),
        }
    }
}
