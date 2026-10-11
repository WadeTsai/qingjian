/// 上一次查詢留給輸入日誌的摘要：上屏時才知道選了什麼，查詢時才知道看到了什麼，兩頭在這裡接上。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuerySnapshot {
    /// 查詢時的作用域（原始鍵）。
    pub scope: String,

    /// 整段的切分（`'` 連線），糾錯生效時是糾正後的。
    pub pinyin: String,

    /// 拼寫糾錯是否生效。
    pub corrected: bool,

    /// 候選文本按順序（只留前面一段，夠定位選了第幾個）。
    pub candidates: Vec<String>,

    /// 候選順序經過了神經重排。
    pub rescored: bool,
}

impl QuerySnapshot {
    /// 最多留多少條候選文本來定位「選了第幾個」。
    pub const MAX_CANDIDATES: usize = 32;
}
