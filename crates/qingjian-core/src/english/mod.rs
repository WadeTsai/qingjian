//! 英文模式（Caps Lock 亮著）的候選：敲的字母不當拼音，按英文詞表給精確詞、字首補全和拼錯一個字母的糾正。
//!
//! 空格與標點仍原樣上屏敲的字母，候選只在按 Tab 或方向鍵選中時才用上，所以候選再多也不礙事。

mod edit;

pub use edit::within_one_edit;
use qingjian_dictionary::WordList;

/// 拼錯糾正至少要幾個字母：三個字母以內一處編輯能變出太多詞，只補全不糾正。
pub const MIN_CORRECTION_LETTERS: usize = 4;

/// 按敲的字母給英文候選：精確詞（按詞表裡的大小寫）第一，然後是字首補全、再是差一處編輯的詞，
/// 後兩組各自按使用者選過的次數、詞頻排序。大小寫跟著敲的走：`Compa` 出 Company，`COMPA` 出 COMPANY。
///
/// `lists` 是若干詞表，前面的優先（使用者自己打過的詞在前、隨包詞表在後）：精確詞取第一個有的，補全與糾正合在一起排。
pub fn suggest(
    lists: &[&WordList],
    typed: &str,
    weight: impl Fn(&str) -> u32,
    limit: usize,
) -> Vec<String> {
    let code = typed.to_ascii_lowercase();
    if code.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut result: Vec<String> = Vec::with_capacity(limit);
    if let Some(word) = lists.iter().find_map(|words| words.get(&code)) {
        result.push(adapt_case(word, typed));
    }
    let ranked = |mut hits: Vec<(&str, u32)>, result: &mut Vec<String>| {
        hits.sort_by(|a, b| {
            weight(b.0)
                .cmp(&weight(a.0))
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.0.cmp(b.0))
        });
        for (word, _) in hits {
            if result.len() >= limit {
                break;
            }
            let word = adapt_case(word, typed);
            if !result.contains(&word) {
                result.push(word);
            }
        }
    };
    let entries = || lists.iter().flat_map(|words| words.entries());
    let completions: Vec<(&str, u32)> = entries()
        .filter(|(entry_code, _, _)| entry_code.len() > code.len() && entry_code.starts_with(&code))
        .map(|(_, word, frequency)| (word, frequency))
        .collect();
    ranked(completions, &mut result);
    // 帶數字或符號的（`foo1`、`x_y`）是識別符號不是拼錯的詞，不去猜
    let letters_only = code.bytes().all(|b| b.is_ascii_lowercase());
    if letters_only && code.len() >= MIN_CORRECTION_LETTERS && result.len() < limit {
        let corrections: Vec<(&str, u32)> = entries()
            .filter(|(entry_code, _, _)| {
                entry_code.len().abs_diff(code.len()) <= 1
                    && !entry_code.starts_with(&code)
                    && within_one_edit(entry_code, &code)
            })
            .map(|(_, word, frequency)| (word, frequency))
            .collect();
        ranked(corrections, &mut result);
    }
    result
}

/// 詞表裡的寫法按敲的大小寫調整：全大寫就全大寫，首字母大寫就首字母大寫，其餘照詞表（iPhone 仍是 iPhone）。
fn adapt_case(word: &str, typed: &str) -> String {
    let mut chars = typed.chars();
    let first_upper = chars.next().is_some_and(|c| c.is_ascii_uppercase());
    let rest_upper = typed.len() >= 2 && typed.chars().all(|c| !c.is_ascii_lowercase());
    if first_upper && rest_upper {
        word.to_ascii_uppercase()
    } else if first_upper {
        let mut out = String::with_capacity(word.len());
        let mut word_chars = word.chars();
        if let Some(c) = word_chars.next() {
            out.extend(c.to_uppercase());
        }
        out.push_str(word_chars.as_str());
        out
    } else {
        word.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> WordList {
        WordList::parse(
            "company\tcompany\t900\ncompare\tcompare\t500\ncompass\tcompass\t300\ncomma\tcomma\t100\n\
             iPhone\tiphone\t800\nhello\thello\t1000\nhollow\thollow\t200\nhelp\thelp\t700\n",
        )
        .unwrap()
    }

    #[test]
    fn exact_then_completions_then_corrections() {
        let list = words();
        assert_eq!(
            suggest(&[&list], "comp", |_| 0, 9),
            ["company", "compare", "compass"]
        );
        // 精確詞第一，詞表寫法優先
        assert_eq!(suggest(&[&list], "iphone", |_| 0, 9), ["iPhone"]);
        // 拼錯一個字母：helo → hello / help（差一處），hollow 差兩處不算
        assert_eq!(suggest(&[&list], "helo", |_| 0, 9), ["hello", "help"]);
        // 三個字母以內不糾正；帶數字的是識別符號，也不糾正
        assert!(suggest(&[&list], "hlo", |_| 0, 9).is_empty());
        assert!(suggest(&[&list], "hel1", |_| 0, 9).is_empty());
        assert!(suggest(&[&list], "", |_| 0, 9).is_empty());
    }

    #[test]
    fn user_choices_and_limit_shape_the_order() {
        let list = words();
        let weight = |t: &str| u32::from(t == "compass");
        assert_eq!(suggest(&[&list], "comp", weight, 2), ["compass", "company"]);
    }

    #[test]
    fn case_follows_the_typed_letters() {
        let list = words();
        assert_eq!(suggest(&[&list], "Comp", |_| 0, 1), ["Company"]);
        assert_eq!(suggest(&[&list], "COMP", |_| 0, 1), ["COMPANY"]);
        assert_eq!(suggest(&[&list], "Iphone", |_| 0, 1), ["IPhone"]);
        assert_eq!(adapt_case("hello", "H"), "Hello");
    }

    #[test]
    fn earlier_lists_win_for_exact_words_and_pool_for_completions() {
        let user = WordList::parse("gist\tgist\t3\n").unwrap();
        let main = WordList::parse("gist\tgist\t0\ngiant\tgiant\t900\ngift\tgift\t500\n").unwrap();
        // 使用者表裡的 gist 精確命中，補全從兩張表合起來按詞頻排
        assert_eq!(
            suggest(&[&user, &main], "gi", |_| 0, 9),
            ["giant", "gift", "gist"]
        );
        // 精確詞後面跟著差一處編輯的 gift
        assert_eq!(suggest(&[&user, &main], "gist", |_| 0, 9), ["gist", "gift"]);
        assert_eq!(suggest(&[&main], "gis", |_| 0, 9), ["gist"]);
    }
}
