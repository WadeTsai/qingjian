use serde::{Deserialize, Serialize};

/// 一次輸入會話的標識。DLL 每進入一個 TSF 文件（`ITfContext`）就開一個會話，Server 按它分派狀態。
///
/// 由 DLL 分配，程序內單調遞增即可；Server 只做鍵，不解釋其數值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SessionId(pub u64);
