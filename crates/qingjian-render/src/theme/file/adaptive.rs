//! 隨外觀變的值：寫一個值兩種外觀共用，或寫 `{ "light": …, "dark": … }` 各給一個。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum Adaptive<T> {
    /// 淺色與深色各一個。
    Pair { light: T, dark: T },

    /// 兩種外觀共用。
    Same(T),
}

impl<T: Copy> Adaptive<T> {
    pub(crate) fn get(&self, dark: bool) -> T {
        match self {
            Self::Pair { light, dark: value } => {
                if dark {
                    *value
                } else {
                    *light
                }
            }
            Self::Same(value) => *value,
        }
    }
}
