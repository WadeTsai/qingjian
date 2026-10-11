/// 一個音節裡的一處敲錯是哪一類，決定它在詞圖裡的代價。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypoKind {
    /// 相鄰兩鍵敲反了（`shou` → `shuo`）。
    Transpose,

    /// 敲到了旁邊的鍵（`ni` → `mi`，n 與 m 相鄰）。不相鄰的鍵不算敲錯（那是讀音問題，模糊音管）。
    Substitute,

    /// 多敲了一個鍵（`gang` → `gan`）。
    Extra,

    /// 少敲了一個鍵（`gan` → `guan`）。
    Missing,
}

impl TypoKind {
    /// 這類敲錯的預設代價（[`TypoCosts::DEFAULT`]）：變體表裡同一寫法來自幾類敲錯時按它留代價最低的一類。
    /// 詞圖打分用引擎當前的 [`TypoCosts`]，不用這個。
    ///
    /// [`TypoCosts`]: super::super::TypoCosts
    /// [`TypoCosts::DEFAULT`]: super::super::TypoCosts::DEFAULT
    pub fn cost(self) -> f64 {
        super::super::TypoCosts::DEFAULT.cost(self)
    }
}
