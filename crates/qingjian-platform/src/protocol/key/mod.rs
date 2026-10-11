//! 按鍵相關的協議型別：一次按鍵（[`KeyEvent`]）、它的修飾鍵（[`KeyModifiers`]）、Server 的處置（[`KeyOutcome`]）。

mod event;
mod modifiers;
mod outcome;

pub use event::KeyEvent;
pub use modifiers::KeyModifiers;
pub use outcome::KeyOutcome;
