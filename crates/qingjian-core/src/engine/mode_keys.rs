//! 字首模式鍵：表示式 / 問字模式的入口字母，以及 `?` 是否也當問字入口。

use serde::{Deserialize, Serialize};

use crate::shortcut::EXPRESSION_PREFIX;
use crate::zhuyin::ZhuyinLayout;

/// 問字模式的標點入口：[`ModeKeys::question_mark`] 開啟時任何配置下 `?` 開頭都進問字模式，也是英文模式下唯一的入口。
pub const QUESTION_PREFIX: char = '?';

/// 字首模式鍵，配置檔案 `[shortcut]` 分節。
///
/// 搜狗 / 微軟那一家的做法：用不能開頭拼任何音節的字母（`v` `u` `i`）一鍵進模式，
/// 不要修飾鍵，中文模式下零衝突。預設 `v` 表示式、`u` 問字（含 Unicode 碼點）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModeKeys {
    /// 表示式模式字首：`v1+2`、`v123`。
    pub expression: char,

    /// 問字模式字首：`usangemu`（三個木）、`u4e00`（碼點）。
    pub question: char,

    /// `?` 開頭是否也進問字模式。預設關：沒在組句時敲的問號就是問號；
    /// 開著時緩衝區為空敲 `?` 先進問字，後面跟字母才是問題，跟別的鍵還原成問號。
    pub question_mark: bool,
}

impl Default for ModeKeys {
    fn default() -> Self {
        Self {
            expression: EXPRESSION_PREFIX,
            question: 'u',
            question_mark: false,
        }
    }
}

impl ModeKeys {
    /// 能當模式鍵的字母：不是任何拼音音節的開頭。
    pub const CANDIDATES: [char; 3] = ['v', 'u', 'i'];

    /// 雙拼下 v / u / i 都是音節鍵，模式鍵換成對應的大寫字母（Shift+V / Shift+U，搜狗 / 微軟的做法）。
    pub fn shifted(self) -> Self {
        Self {
            expression: self.expression.to_ascii_uppercase(),
            question: self.question.to_ascii_uppercase(),
            question_mark: self.question_mark,
        }
    }

    /// 去掉字母模式鍵：形碼下每個字母都是字根鍵，大寫也讓位，只剩 `?`（開著的話）進問字。
    pub fn letterless(self) -> Self {
        Self {
            expression: '\0',
            question: '\0',
            question_mark: self.question_mark,
        }
    }

    /// 兩個鍵都合法且互不相同。不合法的配置整個退回預設，不做一半。
    pub fn is_valid(&self) -> bool {
        self.expression != self.question
            && Self::CANDIDATES.contains(&self.expression)
            && Self::CANDIDATES.contains(&self.question)
    }

    /// 非法配置退回預設（`?` 開關照舊保留）。
    pub fn sanitized(self) -> Self {
        if self.is_valid() {
            self
        } else {
            Self {
                question_mark: self.question_mark,
                ..Self::default()
            }
        }
    }

    /// `zhuyin` 是開著的注音排列：入口字母在那套排列裡是符號鍵時就不是入口。
    pub fn is_expression(&self, input: &str, zhuyin: Option<ZhuyinLayout>) -> bool {
        input.starts_with(self.expression)
            && zhuyin.is_none_or(|layout| layout.map_key(self.expression).is_none())
    }

    pub fn is_question(&self, input: &str, zhuyin: Option<ZhuyinLayout>) -> bool {
        (input.starts_with(self.question)
            && zhuyin.is_none_or(|layout| layout.map_key(self.question).is_none()))
            || self.is_question_mark(input)
    }

    /// 是否以 `?` 進的問字模式：開關關著時 `?` 不是入口。
    fn is_question_mark(&self, input: &str) -> bool {
        self.question_mark && input.starts_with(QUESTION_PREFIX)
    }

    /// 問字模式下字首之後的部分。不在問字模式時原樣返回。
    pub fn question_body<'a>(&self, input: &'a str, zhuyin: Option<ZhuyinLayout>) -> &'a str {
        if self.is_question_mark(input) {
            &input[QUESTION_PREFIX.len_utf8()..]
        } else if self.is_question(input, zhuyin) {
            &input[self.question.len_utf8()..]
        } else {
            input
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_v_and_u_and_question_mark_is_off() {
        let keys = ModeKeys::default();
        assert!(keys.is_expression("v12", None));
        assert!(keys.is_question("usangemu", None));
        assert!(!keys.is_question("?sangemu", None));
        assert_eq!(keys.question_body("usangemu", None), "sangemu");
        assert_eq!(keys.question_body("?sangemu", None), "?sangemu");
        assert!(!keys.is_question("nihao", None));
    }

    #[test]
    fn question_mark_is_an_alias_only_when_switched_on() {
        let keys = ModeKeys {
            question_mark: true,
            ..ModeKeys::default()
        };
        assert!(keys.is_question("?sangemu", None));
        assert_eq!(keys.question_body("?sangemu", None), "sangemu");
        // 雙拼下字母鍵換成大寫，`?` 開關跟著走
        let shifted = keys.shifted();
        assert!(!shifted.is_question("usangemu", None));
        assert!(shifted.is_question("Usangemu", None));
        assert!(shifted.is_expression("V1+2", None));
        assert_eq!(shifted.question_body("Usangemu", None), "sangemu");
        assert!(shifted.is_question("?sangemu", None));
        assert!(!ModeKeys::default().shifted().is_question("?x", None));
    }

    #[test]
    fn letterless_leaves_only_the_question_mark() {
        let keys = ModeKeys {
            question_mark: true,
            ..ModeKeys::default()
        };
        let letterless = keys.letterless();
        assert!(!letterless.is_expression("v1+2", None));
        assert!(!letterless.is_question("usangemu", None));
        assert!(letterless.is_question("?sangemu", None));
        assert_eq!(letterless.question_body("?sangemu", None), "sangemu");
        assert!(!ModeKeys::default().letterless().is_question("?x", None));
    }

    #[test]
    fn invalid_combinations_fall_back_to_defaults() {
        let same = ModeKeys {
            expression: 'v',
            question: 'v',
            question_mark: true,
        };
        assert!(!same.is_valid());
        assert_eq!(
            same.sanitized(),
            ModeKeys {
                question_mark: true,
                ..ModeKeys::default()
            }
        );
        let pinyin_initial = ModeKeys {
            expression: 'v',
            question: 'z',
            question_mark: false,
        };
        assert_eq!(pinyin_initial.sanitized(), ModeKeys::default());
        let swapped = ModeKeys {
            expression: 'i',
            question: 'v',
            question_mark: false,
        };
        assert!(swapped.is_valid());
        assert!(swapped.is_question("v4e00", None));
    }

    #[test]
    fn deserializes_from_single_character_strings() {
        let keys: ModeKeys =
            toml::from_str("expression = \"i\"\nquestion = \"u\"\nquestion_mark = true\n").unwrap();
        assert_eq!(keys.expression, 'i');
        assert_eq!(keys.question, 'u');
        assert!(keys.question_mark);
        let old: ModeKeys = toml::from_str("expression = \"v\"\nquestion = \"u\"\n").unwrap();
        assert!(!old.question_mark);
    }
}
