use super::Unit;
use crate::parser::{Segmentation, Syllable};

/// 一段雙拼鍵解碼的結果：能解的單元 + 解不動的尾巴。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decoded {
    /// 解出來的單元，按敲鍵順序。
    units: Vec<Unit>,

    /// 從第一個配不出音節的位置起的原始鍵，原樣顯示、不參與候選。
    tail: String,

    /// 單元的全拼用 `'` 連成的串（分隔符單元不佔位置），直接餵給全拼切分。
    pinyin: String,
}

impl Decoded {
    pub fn new(units: Vec<Unit>, tail: String) -> Self {
        let mut pinyin = String::with_capacity(units.len() * 6);
        for unit in units.iter().filter(|u| !u.is_separator()) {
            if !pinyin.is_empty() {
                pinyin.push('\'');
            }
            pinyin.push_str(&unit.pinyin);
        }
        Self {
            units,
            tail,
            pinyin,
        }
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// 解出來的全拼，音節之間是 `'`。
    pub fn pinyin(&self) -> &str {
        &self.pinyin
    }

    /// 解不動的尾巴（原始鍵）。
    pub fn tail(&self) -> &str {
        &self.tail
    }

    /// 全部鍵都解成了完整音節：沒有尾巴、末尾沒有落單的鍵。
    pub fn is_complete(&self) -> bool {
        self.tail.is_empty() && self.units.iter().all(|u| u.complete || u.is_separator())
    }

    /// 末尾是不是一個落單的聲母鍵：這時敲 `;`（微軟 / 搜狗的 ing）應該進緩衝區而不是當標點。
    pub fn pending_initial(&self) -> bool {
        self.tail.is_empty()
            && self
                .units
                .last()
                .is_some_and(|u| !u.complete && !u.is_separator())
    }

    /// 解出來的唯一切分：雙拼兩鍵一音節沒有歧義，不必再走全拼切分（那會把 `zhong` 再拆出 `z… hong` 一類簡拼切法）。
    /// 落單的鍵是殘缺音節。空的（一個單元都沒解出）返回 `None`。
    pub fn segmentation(&self) -> Option<Segmentation> {
        let syllables: Vec<Syllable> = self
            .units
            .iter()
            .filter(|u| !u.is_separator())
            .map(|u| {
                if u.complete {
                    Syllable::complete(&u.pinyin)
                } else {
                    Syllable::partial(&u.pinyin)
                }
            })
            .collect();
        (!syllables.is_empty()).then_some(Segmentation { syllables })
    }

    /// 顯示形式：全拼接上尾巴（`ni'bl`）。
    pub fn marked(&self) -> String {
        match (self.pinyin.is_empty(), self.tail.is_empty()) {
            (_, true) => self.pinyin.clone(),
            (true, false) => self.tail.clone(),
            (false, false) => format!("{}'{}", self.pinyin, self.tail),
        }
    }

    /// [`Self::pinyin`] 開頭 `pinyin_len` 個位元組對應多少個鍵：整單元被蓋住才算，緊跟其後的 `'` 一併算上。
    pub fn keys_for(&self, pinyin_len: usize) -> usize {
        let mut keys = 0;
        let mut position = 0;
        let mut first = true;
        let mut pending_separators = 0;
        for unit in &self.units {
            if unit.is_separator() {
                pending_separators += unit.keys.len();
                continue;
            }
            let start = if first { 0 } else { position + 1 };
            let end = start + unit.pinyin.len();
            if pinyin_len < end {
                break;
            }
            keys += pending_separators + unit.keys.len();
            pending_separators = 0;
            position = end;
            first = false;
        }
        // 消耗到末尾時，後面緊跟的 `'` 也一起吃掉
        if keys > 0 {
            keys += pending_separators;
        }
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::super::Scheme;

    #[test]
    fn keys_follow_consumed_syllables() {
        let decoded = Scheme::Xiaohe.decode("kdfave");
        assert_eq!(decoded.pinyin(), "kai'fa'zhe");
        assert_eq!(decoded.keys_for(0), 0);
        assert_eq!(decoded.keys_for(3), 2);
        assert_eq!(decoded.keys_for(6), 4);
        assert_eq!(decoded.keys_for(10), 6);
        // 只蓋住半個音節不算
        assert_eq!(decoded.keys_for(5), 2);
    }

    #[test]
    fn partial_last_unit_counts_one_key() {
        let decoded = Scheme::Xiaohe.decode("kdf");
        assert_eq!(decoded.keys_for(5), 3);
        assert_eq!(decoded.keys_for(3), 2);
    }

    #[test]
    fn separators_go_with_the_syllable_before_them() {
        let decoded = Scheme::Xiaohe.decode("xi'an");
        assert_eq!(decoded.pinyin(), "xi'an");
        assert_eq!(decoded.keys_for(2), 3);
        assert_eq!(decoded.keys_for(5), 5);
        assert!(decoded.is_complete());
    }

    #[test]
    fn segmentation_mirrors_units() {
        let decoded = Scheme::Xiaohe.decode("kdf'ni");
        let segmentation = decoded.segmentation().unwrap();
        assert_eq!(segmentation.to_string(), "kai f… ni");
        assert!(Scheme::Xiaohe.decode("bl").segmentation().is_none());
    }

    #[test]
    fn pending_initial_wants_a_final() {
        assert!(Scheme::Microsoft.decode("x").pending_initial());
        assert!(Scheme::Microsoft.decode("nix").pending_initial());
        assert!(!Scheme::Microsoft.decode("ni").pending_initial());
        assert!(!Scheme::Microsoft.decode("").pending_initial());
        assert!(!Scheme::Microsoft.decode("nibl").pending_initial());
    }
}
