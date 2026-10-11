//! 注音鍵盤排列：按鍵 → 注音符號元件。排列在 [`ZhuyinLayout`]（大千 / 倚天），元件在 [`Component`]。

mod component;
mod keyboard;

pub use component::Component;
pub use keyboard::ZhuyinLayout;
