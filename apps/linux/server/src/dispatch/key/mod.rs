//! 按鍵處理：鍵碼 / 字元解析在 [`codes`]，分流在 [`input`]，「修飾鍵 + 數字」快捷鍵在 [`shortcut`]，
//! 一次按鍵的結果是 [`Effect`]。

mod codes;
mod effect;
mod input;
mod shortcut;

pub(super) use self::effect::Effect;

/// 先行上屏的文本與原本透傳的字符合並交付，保證應用接收順序與空格數量。
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
