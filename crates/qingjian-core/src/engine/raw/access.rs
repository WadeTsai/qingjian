//! 不查詢候選、不提交、不記錄的原樣文本讀取。
use crate::engine::{Engine, RawPreedit};
use crate::zhuyin;

impl Engine {
    /// 讀取整段尚未上屏的原樣文本，包括游標後的內容，不改變輸入或學習狀態。
    ///
    /// 普通方案保留實際鍵串、大小寫及顯式分隔符，不包含待處理輔碼。
    /// 注音沿用原樣提交的符號輸出：單元內按鍵字首產生的符號數，對映到完整
    /// 單元的同序字元邊界；輕聲重排仍用該邏輯位置，一聲空格不移動顯示游標。
    /// 未知鍵沿用解碼器聚到末尾的行為，游標跟隨其輸出位置，可能不單調。
    /// 首位始終為 0，末位始終為完整文本末尾。
    pub fn raw_preedit(&self) -> RawPreedit {
        if self.is_zhuyin_mode() && !self.english_mode {
            let (decoded, cursor_bytes) =
                zhuyin::decode_at(self.composition.text(), self.composition.cursor());
            RawPreedit {
                text: decoded.marked(),
                cursor_bytes,
            }
        } else {
            let text = self.composition.typed_text();
            let mut cursor_bytes = self.composition.cursor().min(text.len());
            while !text.is_char_boundary(cursor_bytes) {
                cursor_bytes -= 1;
            }
            RawPreedit { text, cursor_bytes }
        }
    }
}
