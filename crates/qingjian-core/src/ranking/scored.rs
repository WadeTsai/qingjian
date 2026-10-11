use std::cmp::Reverse;

use qingjian_dictionary::Match;

/// 一條待排序的詞庫命中。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scored<'a> {
    /// 詞庫命中，含詞、拼音、詞頻與是否精確。
    pub hit: Match<'a>,

    /// 輸入的最後一個音節是完整的，且與詞的對應音節相同（而非字首擴充套件）。
    pub full_last: bool,

    /// 詞覆蓋了輸入開頭多少個字母（不含 `'`）。按字母而不是音節算，不同切分之間才可比：
    /// `xian` 的 先（1 音節）和 西安（2 音節）覆蓋同樣 4 個字母。字首詞（`kaifazhe` → 開發）覆蓋得少。
    pub coverage: usize,

    /// 切分裡非末尾的簡拼音節數（`kai f a` 是 1，`kai fa` 是 0）：越少越像使用者敲的原話，
    /// 否則 `kaifa` 會因 `kai f a` 這種切法讓 開放啊 排到 開放 前面。
    pub abbreviated: usize,

    /// 使用者選擇過的次數，來自 Learner。
    pub weight: u32,

    /// 靠模糊音 / 敲錯變體命中的代價（某個音節與敲的不同；0 是敲的原樣）：從上下文得分里扣掉，
    /// 預選時詞頻按 e^(-代價) 打折，同分排在原樣命中後面。
    pub penalty: f64,
}

/// 預選鍵：結構項之後按使用者選擇次數與詞庫詞頻，命中太多時先用它砍到夠排的量（不必算上下文得分）。
/// 打包成一個整數、越大越靠前：單字母簡拼一鍵命中幾萬條，逐條比元組太慢。
/// 位從高到低：精確(1) 覆蓋字母數(8) 簡拼數的補(8) 末音節完整(1) 選擇次數(32) 打折後詞頻(32) 原樣命中(1) 字數的補(8)；
/// 與 [`SortKey`] 的前幾項同序，只是不拿文本做最後的平手項（預選邊界上的平手誰留下無所謂）。
pub type PreselectKey = u128;

/// 排序鍵，越小越靠前。元組的順序即排序規則，見模組文件；第五項是同輸入串下的選擇次數，
/// 第六項是上下文得分（毫分，整數才能比較）。文本借自詞庫，鍵可以脫離 `Scored` 存放。
pub type SortKey<'a> = (
    Reverse<bool>,
    Reverse<usize>,
    usize,
    Reverse<bool>,
    Reverse<u32>,
    Reverse<i64>,
    bool,
    usize,
    &'a str,
);

impl<'a> Scored<'a> {
    /// 命中不是敲的原樣（模糊音或敲錯變體）。
    pub fn altered(&self) -> bool {
        self.penalty > 0.0
    }

    pub(super) fn preselect_key(&self) -> PreselectKey {
        let frequency = if self.altered() {
            (f64::from(self.hit.frequency) * (-self.penalty).exp()) as u32
        } else {
            self.hit.frequency
        };
        let chars = u128::from(u8::try_from(self.hit.text.chars().count()).unwrap_or(u8::MAX));
        let coverage = u128::from(u8::try_from(self.coverage).unwrap_or(u8::MAX));
        let abbreviated = u128::from(u8::try_from(self.abbreviated).unwrap_or(u8::MAX));
        (u128::from(self.hit.exact) << 90)
            | (coverage << 82)
            | ((0xFF - abbreviated) << 74)
            | (u128::from(self.full_last) << 73)
            | (u128::from(self.weight) << 41)
            | (u128::from(frequency) << 9)
            | (u128::from(!self.altered()) << 8)
            | (0xFF - chars)
    }

    /// `choice` 是同輸入串下的選擇次數，`score` 是上下文得分（log 機率，已含使用者加分與模糊音 / 敲錯扣分）。
    pub(super) fn key(&self, choice: u32, score: f64) -> SortKey<'a> {
        (
            Reverse(self.hit.exact),
            Reverse(self.coverage),
            self.abbreviated,
            Reverse(self.full_last),
            Reverse(choice),
            Reverse((score * 1000.0).round() as i64),
            self.altered(),
            self.hit.text.chars().count(),
            self.hit.text,
        )
    }
}
