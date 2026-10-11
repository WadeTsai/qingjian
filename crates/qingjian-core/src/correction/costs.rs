use super::TypoKind;

/// 敲錯糾正的代價（log 機率的扣分）。預設值是現在用的那組，回放調參（`qingjian-cli --tune`）時可以整組換掉，
/// 引擎與殼只用預設值。
///
/// 音節級的四類代價疊在詞的得分上：候選拼音與敲的不同時，要比原樣的解釋好這麼多才排得過。
/// 換位與相鄰鍵最常見、代價最低；多敲少敲稍貴。數值按「一個音節敲錯的先驗約 1%，再分攤到它的十幾個變體上」定，
/// 與整段一處編輯的代價同一量級；定太低時常用詞會藉著個人詞頻從敲錯邊擠掉使用者真要的生僻詞。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypoCosts {
    /// 相鄰兩鍵敲反了（`shou` → `shuo`）。
    pub transpose: f64,

    /// 敲到了旁邊的鍵（`ni` → `mi`）。
    pub substitute: f64,

    /// 多敲了一個鍵（`gang` → `gan`）。
    pub extra: f64,

    /// 少敲了一個鍵（`gan` → `guan`）。
    pub missing: f64,

    /// 個人敲錯表最多給一條邊減多少代價：減到只剩 1.0 左右，敲錯的解釋仍要比原樣好一點才排上來。
    pub discount_cap: f64,

    /// 整段一處編輯的糾錯代價：糾正後的整句得分要比原樣轉出的高出這麼多才糾。
    /// 相當於「敲錯一個鍵」的先驗約 1/150；原樣是合法簡拼（`nhao` → 你好）時兩邊路徑一樣，糾正不會贏。
    pub correction_penalty: f64,

    /// 整段糾錯裡相鄰換位比別的編輯便宜多少：一個位置只有一種換法、卻有七八個相鄰鍵可以換錯，
    /// 同樣能說通時換位的解釋更可能（`mignti` 換位成 `mingti` 而不是換字母成 `mianti`）。
    pub correction_transpose_discount: f64,
}

impl TypoCosts {
    /// 現在用的那組。
    pub const DEFAULT: Self = Self {
        transpose: 5.0,
        substitute: 5.0,
        extra: 5.5,
        missing: 5.5,
        discount_cap: 3.0,
        correction_penalty: 5.0,
        correction_transpose_discount: 1.0,
    };

    /// 這類敲錯的基礎代價。
    pub fn cost(&self, kind: TypoKind) -> f64 {
        match kind {
            TypoKind::Transpose => self.transpose,
            TypoKind::Substitute => self.substitute,
            TypoKind::Extra => self.extra,
            TypoKind::Missing => self.missing,
        }
    }

    /// 詞圖裡一條敲錯邊的代價：類別的基礎代價按個人敲錯表打折（見 [`Self::discounted`]）。
    pub fn typo_cost(&self, kind: TypoKind, accepted: u32) -> f64 {
        self.discounted(self.cost(kind), accepted)
    }

    /// 整段一處編輯的糾錯代價：相鄰換位先減 [`Self::correction_transpose_discount`]，再按個人敲錯表打折。
    pub fn correction_cost(&self, transpose: bool, accepted: u32) -> f64 {
        let base = if transpose {
            self.correction_penalty - self.correction_transpose_discount
        } else {
            self.correction_penalty
        };
        self.discounted(base, accepted)
    }

    /// `base` 代價減去個人折扣：折扣 = min(ln(1 + 接受過的次數), [`Self::discount_cap`])。
    pub fn discounted(&self, base: f64, accepted: u32) -> f64 {
        base - (1.0 + f64::from(accepted)).ln().min(self.discount_cap)
    }
}

impl Default for TypoCosts {
    fn default() -> Self {
        Self::DEFAULT
    }
}
