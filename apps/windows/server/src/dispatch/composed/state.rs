//! 組句展示狀態的列舉。

use qingjian_core::CandidateLayout;
use qingjian_platform::protocol::PreeditSegment;

/// 當前組句緩衝對應的展示狀態。緩衝變化時重建，導航只挪高亮，雲端詞非同步並進 `layout`。
pub(crate) enum Composed {
    /// 正常查到候選。
    Candidates {
        /// preedit 分段。
        preedit: Vec<PreeditSegment>,

        /// 游標在拼音行裡的字元位置。
        cursor: usize,

        /// 雙拼「輸入框顯示原始按鍵」開著時，應用輸入框裡改放這一串；關著為 `None`，輸入框與拼音行一樣。
        typed_keys: Option<TypedKeys>,

        /// 本地 + 雲端槽位的候選佈局。
        layout: CandidateLayout,
    },

    /// 查詢失敗（如上屏後剩下不可切分的殘餘）：顯示原始拼音、無候選。
    /// 不能回空幀，否則組句非空卻沒有候選窗，只有 clear / 切輸入法能解。
    Raw {
        /// 原始拼音。
        text: String,

        /// 游標字元位置。
        cursor: usize,
    },
}

/// 應用輸入框裡的原始按鍵與游標（字元下標）。
pub(crate) struct TypedKeys {
    pub(crate) text: String,
    pub(crate) cursor: usize,
}
