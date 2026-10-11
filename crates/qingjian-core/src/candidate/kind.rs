use serde::{Deserialize, Serialize};

/// 候選的來源型別，平臺層可據此區別顯示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateKind {
    /// 中文詞庫裡的詞。
    Chinese,

    /// 形碼碼錶裡的字詞（五筆）。編碼平鋪，沒有音節：`syllables` 為空，上屏吃掉整段編碼。
    Code,

    /// 英文詞表裡的詞（中英混輸），上屏時吃掉整段輸入。
    English,

    /// 雲聯想給出的詞：帶全拼音節，上屏後記成使用者詞。
    Cloud,

    /// 快捷候選（日期 / 時間 / 星期 / 算式結果 / 中文數字），由輸入直接算出，上屏時吃掉整段作用域、不記學習。
    Shortcut,

    /// 自定義短語的固定位置，從 1 開始。
    Custom(usize),

    /// 按候選詞配的 emoji，音節與那個詞相同；上屏按音節消耗拼音，不記學習。
    Emoji,

    /// 離線整句轉換的結果（多個詞拼成），帶全部音節；上屏按音節消耗拼音，路徑上的詞逐條記入個人 n-gram，不記詞頻。
    Sentence,

    /// 模型直接按按鍵生成的整句（`yongdockerbushuhenfangbian` → 用docker部署很方便）：
    /// 詞圖讀不通整段輸入時的兜底來源，沒有音節對齊，上屏吃掉整段作用域、不記學習。
    Generated,
}
