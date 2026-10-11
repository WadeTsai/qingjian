//! Server ↔ DLL 的 IPC 協議型別。
//!
//! Windows 的 TSF DLL 會被載入進每一個應用程序，所以 [`qingjian_core::Engine`] 不能待在 DLL 裡，
//! 得跑在獨立的 Server 程序；DLL 只做 IPC，把系統按鍵翻譯成 [`ClientMessage`] 發給 Server，
//! 把 Server 回的 [`ServerMessage`] 畫到候選視窗。結構與 Weasel（WeaselServer）/ 水杉（Server 程序）一致。
//!
//! 這裡的型別兩端共用，必須可序列化（serde）。協議只描述「按鍵進、要畫什麼出」這一層，
//! 不復制 Core 的排序 / 詞庫邏輯：候選直接用 [`qingjian_core::CandidateList`]，preedit 分段用
//! [`PreeditSegment`]（Core 內部的 `MarkedSegment` 的可序列化映象，避免協議耦合 Core 的內部列舉）。

mod client;
mod codec;
mod indicator;
mod screen_rect;
mod server;
mod session;

/// 協議版本，DLL 開會話時帶上。加訊息 / 改欄位語義時 +1，一個版本週期只升一次（本週期已升過就不再升；
/// Linux 外掛寫死了這個數，升了一起改）；Server 只對不上時記警告（老 DLL 在沒重啟的
/// 應用裡還會活很久，serde 的預設欄位 / 忽略未知欄位讓兩邊仍能對話）。
///
/// **加列舉變體不在「仍能對話」之列**：`qingjian_core::Candidate` 是線上格式的一部分（見本模組文件），
/// 給它加一個 `kind` 變體，老 DLL 解不出來會整條幀失敗、按鍵直接放行——測試時看到的「輸入法突然只出英文」
/// 就是這麼來的（`unknown variant `Code``）。加變體必須同時 +1 並重裝 DLL，否則連警告都不會有。
pub const PROTOCOL_VERSION: u32 = 9;

/// 從哪個協議版本起 DLL 會在 `OpenSession` 後阻塞讀一條 [`ServerMessage::SessionOpened`]。
/// 門檻是固定值而不是當前版本：以後版本再升，沒重啟的應用裡那些舊 DLL 仍在等這條回包，
/// 不回它們會卡在 `open()` 裡（宿主 UI 執行緒）。
pub const SESSION_OPENED_SINCE: u32 = 6;

pub mod frame;
pub mod key;

pub use client::ClientMessage;
pub use codec::{CodecError, DEFAULT_PIPE_NAME, read_message, write_message};
pub use frame::{Frame, PreeditKind, PreeditSegment};
pub use indicator::{IndicatorCommand, IndicatorState};
pub use key::{KeyEvent, KeyModifiers, KeyOutcome};
pub use screen_rect::ScreenRect;
pub use server::{InputSettings, ServerMessage};
pub use session::SessionId;
