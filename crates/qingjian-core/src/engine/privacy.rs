//! 私密輸入：密碼框、瀏覽器無痕視窗這類應用宣告「別記」的地方。殼判定（macOS 的 Secure Input 直接不組句；
//! Windows 的 `KEYBOARD_DISABLED` compartment 同樣不組句，`IS_PRIVATE` / 密碼類輸入範圍則照常組句但走這裡），
//! Core 這一側：不學習、不記輸入日誌、不發雲端（聯想 / 翻譯 / 釋義兜底）；排序仍用已有的個人資料。

use super::Engine;

impl Engine {
    /// 進入 / 離開私密輸入。殼在焦點落到私密輸入框（或離開它）時調；跨會話切換焦點時按各會話的狀態重設。
    /// 此方法只設置寫入開關，不清組句；已確認的隱私能力邊界應先呼叫 [`Self::discard_input`]，
    /// 避免私密快取被普通輸入繼續使用。恢復另一個獨立會話時不需要丟棄該會話的輸入。
    pub fn set_private(&mut self, private: bool) {
        if self.private == private {
            return;
        }
        self.private = private;
        self.learner.set_private(private);
        self.logger.set_muted(private);
        if private {
            // 在飛的雲結果不能再顯示，前文也不能留
            self.prediction_sequence += 1;
            self.rescoring_before = None;
        }
    }

    pub fn is_private(&self) -> bool {
        self.private
    }
}
