use qingjian_dictionary::{SyllablePattern, canonical_syllable};
use serde::{Deserialize, Serialize};

use super::Expanded;
use crate::parser;
use crate::ranking::FUZZY_PENALTY;

/// 模糊音開關，配置檔案 `[fuzzy]` 分節；預設全關。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FuzzyRules {
    /// z ↔ zh。
    pub z_zh: bool,

    /// c ↔ ch。
    pub c_ch: bool,

    /// s ↔ sh。
    pub s_sh: bool,

    /// n ↔ l。
    pub n_l: bool,

    /// f ↔ h。
    pub f_h: bool,

    /// l ↔ r。
    pub l_r: bool,

    /// an ↔ ang（含 ian / iang、uan / uang）。
    pub an_ang: bool,

    /// en ↔ eng。
    pub en_eng: bool,

    /// in ↔ ing。
    pub in_ing: bool,
}

impl FuzzyRules {
    /// 全開，測試和 CLI 用。
    pub const ALL: Self = Self {
        z_zh: true,
        c_ch: true,
        s_sh: true,
        n_l: true,
        f_h: true,
        l_r: true,
        an_ang: true,
        en_eng: true,
        in_ing: true,
    };

    pub fn any(&self) -> bool {
        *self != Self::default()
    }

    /// 全部規則的配置鍵名，與 `[fuzzy]` 分節的欄位名一致；選單按這個順序列出。
    pub const NAMES: [&'static str; 9] = [
        "z_zh", "c_ch", "s_sh", "n_l", "f_h", "l_r", "an_ang", "en_eng", "in_ing",
    ];

    /// 按名字開一條規則（`z-zh` / `z_zh` / `zzh` 都認），CLI 引數用；不認識返回 `false`。
    pub fn enable(&mut self, name: &str) -> bool {
        self.set(name, true)
    }

    /// 按名字設定一條規則的開關；不認識返回 `false`。
    pub fn set(&mut self, name: &str, on: bool) -> bool {
        match self.flag_mut(name) {
            Some(flag) => {
                *flag = on;
                true
            }
            None => false,
        }
    }

    /// 按名字查一條規則是否開著；不認識返回 `false`。
    pub fn is_on(&self, name: &str) -> bool {
        let mut copy = *self;
        copy.flag_mut(name).is_some_and(|flag| *flag)
    }

    fn flag_mut(&mut self, name: &str) -> Option<&mut bool> {
        let normalized: String = name
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_ascii_lowercase();
        Some(match normalized.as_str() {
            "zzh" => &mut self.z_zh,
            "cch" => &mut self.c_ch,
            "ssh" => &mut self.s_sh,
            "nl" => &mut self.n_l,
            "fh" => &mut self.f_h,
            "lr" => &mut self.l_r,
            "anang" => &mut self.an_ang,
            "eneng" => &mut self.en_eng,
            "ining" => &mut self.in_ing,
            _ => return None,
        })
    }

    /// 把一串模式擴充套件成每個位置的多種寫法（第一種是使用者敲的，代價 0；模糊音寫法扣 [`FUZZY_PENALTY`]）。
    pub fn expand(&self, patterns: &[SyllablePattern<'_>]) -> Expanded {
        Expanded::new(patterns.iter().map(|p| {
            let forms = self
                .alternatives(*p)
                .into_iter()
                .enumerate()
                .map(|(index, text)| (text, if index == 0 { 0.0 } else { FUZZY_PENALTY }))
                .collect();
            (forms, p.complete)
        }))
    }

    /// `syllable` 是不是 `typed` 按當前規則的一種模糊寫法（不含它自己）。
    pub fn is_variant(&self, typed: &str, syllable: &str) -> bool {
        canonical_syllable(typed) != canonical_syllable(syllable)
            && self
                .alternatives(SyllablePattern::complete(typed))
                .iter()
                .any(|form| canonical_syllable(form) == canonical_syllable(syllable))
    }

    /// 一個音節的全部寫法，第一個是敲的原文；沒開模糊音就只有它自己。
    /// 同一位置的寫法互不覆蓋（詞庫多寫法查詢的契約）：字首 `zh` 換成 `z` 後只留 `z`。
    fn alternatives(&self, pattern: SyllablePattern<'_>) -> Vec<String> {
        let mut forms = vec![pattern.text.to_owned()];
        if !self.any() {
            return forms;
        }
        // 規則之間可以疊加（zhen → zeng），跑到不再增長為止；規則集很小，兩輪就收斂
        let mut index = 0;
        while index < forms.len() {
            let current = forms[index].clone();
            for candidate in self.apply_once(&current, pattern.complete) {
                if !forms.contains(&candidate) {
                    forms.push(candidate);
                }
            }
            index += 1;
        }
        if !pattern.complete {
            // 字首之間去覆蓋：被別的更短字首包含的去掉
            let snapshot = forms.clone();
            forms.retain(|f| !snapshot.iter().any(|o| o != f && f.starts_with(o.as_str())));
            // 原文若被去掉了，把覆蓋它的那個放到第一位，保證第一種寫法仍能匹配原文能匹配的
            if forms.first().map(String::as_str) != Some(pattern.text)
                && let Some(position) = forms
                    .iter()
                    .position(|f| pattern.text.starts_with(f.as_str()))
            {
                forms.swap(0, position);
            }
        }
        forms
    }

    /// 對一種寫法各套一次規則得到的新寫法（可能不合法，這裡過濾掉）。
    fn apply_once(&self, form: &str, complete: bool) -> Vec<String> {
        let mut out = Vec::new();
        let mut swap_initial = |a: &str, b: &str| {
            if let Some(rest) = form.strip_prefix(a) {
                out.push(format!("{b}{rest}"));
            } else if let Some(rest) = form.strip_prefix(b) {
                out.push(format!("{a}{rest}"));
            }
        };
        // 先比雙字母聲母，`sh` 不能被當成 `s` 處理
        if self.z_zh {
            swap_initial("zh", "z");
        }
        if self.c_ch {
            swap_initial("ch", "c");
        }
        if self.s_sh {
            swap_initial("sh", "s");
        }
        if self.n_l {
            swap_initial("n", "l");
        }
        if self.f_h {
            swap_initial("f", "h");
        }
        if self.l_r {
            swap_initial("l", "r");
        }
        if complete {
            let mut swap_final = |a: &str, b: &str| {
                if let Some(head) = form.strip_suffix(a) {
                    out.push(format!("{head}{b}"));
                } else if let Some(head) = form.strip_suffix(b) {
                    out.push(format!("{head}{a}"));
                }
            };
            // 先比長的：`ang` 結尾的不能再被當成 `an`
            if self.an_ang {
                swap_final("ang", "an");
            }
            if self.en_eng {
                swap_final("eng", "en");
            }
            if self.in_ing {
                swap_final("ing", "in");
            }
        }
        out.retain(|f| {
            !f.is_empty()
                && f != form
                && if complete {
                    parser::is_syllable(f)
                } else {
                    parser::is_syllable(f) || parser::is_syllable_prefix(f)
                }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forms(rules: &FuzzyRules, pattern: SyllablePattern<'_>) -> Vec<String> {
        rules.alternatives(pattern)
    }

    #[test]
    fn initial_rules_apply_both_ways_and_combine_with_finals() {
        let rules = FuzzyRules::ALL;
        let mut zhen = forms(&rules, SyllablePattern::complete("zhen"));
        zhen.sort();
        assert_eq!(zhen, ["zen", "zeng", "zhen", "zheng"]);
        assert_eq!(forms(&rules, SyllablePattern::complete("lan"))[0], "lan");
        let mut lan = forms(&rules, SyllablePattern::complete("lan"));
        lan.sort();
        assert_eq!(lan, ["lan", "lang", "nan", "nang", "ran", "rang"]);
        // 不合法的寫法不出：hua 沒有 fua
        assert_eq!(forms(&rules, SyllablePattern::complete("hua")), ["hua"]);
    }

    #[test]
    fn only_enabled_rules_apply() {
        let mut rules = FuzzyRules::default();
        assert_eq!(forms(&rules, SyllablePattern::complete("zi")), ["zi"]);
        assert!(rules.enable("z-zh"));
        assert!(!rules.enable("q-x"));
        assert_eq!(
            forms(&rules, SyllablePattern::complete("zi")),
            ["zi", "zhi"]
        );
        assert_eq!(
            forms(&rules, SyllablePattern::complete("zhi")),
            ["zhi", "zi"]
        );
        // 韻母規則沒開：fen 只有自己（f/h 也沒開）
        assert_eq!(forms(&rules, SyllablePattern::complete("fen")), ["fen"]);
    }

    #[test]
    fn prefixes_only_get_initial_rules_and_never_overlap() {
        let rules = FuzzyRules::ALL;
        // 字首 zh 換成 z 後 z 已覆蓋 zh，只留 z
        assert_eq!(forms(&rules, SyllablePattern::prefix("zh")), ["z"]);
        assert_eq!(forms(&rules, SyllablePattern::prefix("z")), ["z"]);
        let mut fe = forms(&rules, SyllablePattern::prefix("fe"));
        fe.sort();
        assert_eq!(fe, ["fe", "he"]);
        // 字首不套韻母規則
        assert_eq!(forms(&rules, SyllablePattern::prefix("xin")), ["xin"]);
        let expanded = rules.expand(&[
            SyllablePattern::complete("zi"),
            SyllablePattern::prefix("l"),
        ]);
        let positions = expanded.positions();
        assert_eq!(positions[0][0], SyllablePattern::complete("zi"));
        assert_eq!(positions[0][1], SyllablePattern::complete("zhi"));
        assert_eq!(positions[1].len(), 3); // l n r
        assert!(positions[1].iter().all(|p| !p.complete));
    }
}
