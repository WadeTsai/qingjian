//! Linux 獨立的面板協議擴充套件；Windows ClientMessage/ServerMessage 和版本不變。
mod capabilities;
mod display;
mod event;
mod identity;
mod request;
pub use capabilities::Capabilities;
pub use display::DisplayAcknowledged;
pub use event::LinuxEvent;
pub use identity::DisplayIdentity;
pub use request::LinuxRequest;
pub const LINUX_UI_PROTOCOL: u32 = 3;
