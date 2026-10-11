//! 一次要繪製的組句狀態：preedit 行加候選頁。

pub mod preedit;

pub use preedit::{PreeditKind, PreeditSegment};

use serde::{Deserialize, Serialize};

use qingjian_core::CandidateList;

use crate::{Appearance, LayoutMode, PreeditMode};

/// Server 告訴 DLL「現在螢幕上該是什麼樣」：組句的拼音行、候選頁、高亮與頁碼。
/// 空 [`Frame`]（`preedit` 與 `candidates` 都空）表示沒有在組句，DLL 收起候選視窗。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    /// 組句拼音行的分段，按順序拼成整行。
    pub preedit: Vec<PreeditSegment>,

    /// 拼音顯示在哪（`[general] preedit`）：DLL 按 [`PreeditMode::inline`] 決定要不要往應用裡放行內拼音，
    /// 視窗頂部畫不畫拼音行由 Server 自己按 [`PreeditMode::in_window`] 定。
    #[serde(default)]
    pub preedit_mode: PreeditMode,

    /// 游標在拼音行裡的位置，按 `preedit` 拼接後的字元（`char`）數算。
    pub cursor: usize,

    /// 當前頁的候選（已排好序、不帶譯文由後續 [`super::ServerMessage::Update`] 補）。
    pub candidates: CandidateList,

    /// 當前頁裡高亮的候選下標（頁內，從 0 起）。
    pub highlight: usize,

    /// 當前頁碼（從 0 起）。
    pub page: usize,

    /// 總頁數；翻頁鍵是否可用看它。
    pub page_count: usize,

    /// 候選排布（豎排 / 橫排）。DLL 是純渲染端，佈局由 Server 按 `[general] layout` 配置隨幀下發。
    pub layout: LayoutMode,

    /// 候選視窗外觀（跟隨系統 / 淺色 / 深色）。`System` 由繪製端按當前系統深淺色解析。
    /// 線上欄位名仍是 `theme`（2026-09-18 之前的名字），沒重啟的應用裡的舊 DLL 要求有這個欄位，改名會讓它解不出幀。
    #[serde(rename = "theme")]
    pub appearance: Appearance,

    /// 候選上是否顯示輔碼（`[general] aux_code_show`）：候選帶 `aux_code` 時拼在 annotation 最前面。
    /// 與 `layout` / `theme` 一樣，顯示開關由 Server 按配置隨幀下發。
    #[serde(default)]
    pub aux_code_show: bool,

    /// 整句補全（雲聯想給的整段拼音的整句結果）：畫在 preedit 行右側，按 Tab 上屏。無則 `None`。
    pub sentence: Option<String>,

    /// 螢幕提示（刪候選後的「已刪除…」一句）：畫在 preedit 行下方，顯示到下一次按鍵。無則 `None`。
    /// 不參與 [`is_empty`](Self::is_empty)：單有提示不算在組句，否則空組句也會撐開候選視窗。
    #[serde(default)]
    pub notice: Option<String>,
}

impl Frame {
    /// 沒有在組句：DLL 據此收起候選視窗。提示不算數（見 [`notice`](Self::notice)）。
    pub fn is_empty(&self) -> bool {
        self.preedit.is_empty() && self.candidates.items.is_empty()
    }
}
