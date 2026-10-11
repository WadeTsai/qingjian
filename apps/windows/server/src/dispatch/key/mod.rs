//! 按鍵處理：鍵碼 / 字元解析在 [`codes`]，分流在 [`input`]，「修飾鍵 + 數字」快捷鍵在 [`shortcut`]，
//! 滑鼠點候選視窗在 [`click`]（與數字鍵 / 譯詞鍵同一套上屏），一次按鍵的結果是 [`Effect`]。

mod click;
mod codes;
mod effect;
mod input;
mod shortcut;

pub(super) use self::codes::{ESCAPE, RETURN};
pub(super) use self::effect::Effect;

/// 把先行上屏的文本接到本次結果前面。Windows 放行是同步的、上屏走非同步編輯會話，
/// 分兩步會讓應用先插這個鍵再插詞，所以本該放行的鍵改由我們連同字首一起插入。
fn with_prefix(prefix: Option<String>, effect: Effect, c: char) -> Effect {
    let Some(mut prefix) = prefix else {
        return effect;
    };
    match effect {
        Effect::Changed(commit) => {
            prefix.push_str(commit.as_deref().unwrap_or_default());
        }
        Effect::Navigated => {}
        Effect::Passthrough => prefix.push(c),
    }
    Effect::Changed(Some(prefix))
}

#[cfg(test)]
mod tests;
