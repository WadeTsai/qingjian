use serde::{Deserialize, Serialize};

use super::frame::Frame;
use super::indicator::IndicatorState;
use super::key::KeyOutcome;
use super::session::SessionId;
use crate::config::SwitchKeys;

/// Server 下發給 DLL 的「按鍵行為」設定。
///
/// DLL 跑在每個應用的程序裡，拿不到 Server 那份 [`Config`](crate::Config)，但這兩個值在**按鍵到達之前**
/// 就得知道：單擊切換鍵的判定在 `OnTestKeyUp` 裡做，內建英文模式開關決定要不要登記語言欄按鈕。
/// 所以由 Server 讀配置（它本來就在盯熱載入）經協議下發，DLL 不讀檔案、不查 mtime——
/// `%APPDATA%\Qingjian` 對 AppContainer 裡的商店應用本來也讀不到。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSettings {
    /// 中英切換鍵（`[shortcut] switch_mode`）。
    pub switch_mode: SwitchKeys,

    /// 內建英文模式總開關（`[general] english_mode`）。
    pub english_mode: bool,

    /// 中文模式下 Shift+字母進組句（`[general] shift_letter = "compose"`）。DLL 據此決定沒在組句時
    /// 按住 Shift 敲的字母吃不吃：預設交給應用，開著時送 Server 起一段組句（`⇧C` 接 `pan` 出「C盤」）。
    #[serde(default)]
    pub shift_letter_compose: bool,

    /// 這個會話的應用裡英文模式給不給英文候選（`[general] english_candidates` 與 `[apps]` 白名單
    /// 由 Server 按會話的 exe 名算好）。不給時英文模式與 Caps 下沒在組句的字母 DLL 不吃、原樣交給應用，
    /// 否則 Photoshop 畫布這類沒有文本框的地方收不到 W / H / V 這些單鍵快捷鍵。老 Server 不帶，按給。
    #[serde(default = "english_candidates_default")]
    pub english_candidates: bool,
}

fn english_candidates_default() -> bool {
    true
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            switch_mode: SwitchKeys::default(),
            english_mode: true,
            shift_letter_compose: false,
            english_candidates: true,
        }
    }
}

/// Server 發給 DLL 的訊息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// 對一次 [`super::ClientMessage::OpenSession`] 的答覆：把 DLL 在按鍵到達之前就要知道的
    /// 設定帶過去一次（之後 [`Self::ModeSync`] 的每一拍也帶著，改了配置不用重開會話）。
    ///
    /// **只回過協議版本對得上的 DLL**：老的 `open` 是隻寫不讀，多回一條會被它當成下一次
    /// `Poll` 的應答而報錯（見 Server 側 `handle`）。
    SessionOpened {
        /// 會話標識。
        session: SessionId,

        /// 按鍵行為設定。
        input: InputSettings,
    },

    /// 對一次 [`super::ClientMessage::Key`] 的處理結果。
    KeyResult {
        /// 會話標識。
        session: SessionId,

        /// 這次按鍵吃掉還是放行。
        outcome: KeyOutcome,

        /// 本次要立即上屏的文本（選詞 / 空格上屏 / 標點等）；沒有則為 `None`。
        commit: Option<String>,

        /// 處理後要繪製的組句狀態（preedit + 候選）；空 [`Frame`] 表示收起候選視窗。
        frame: Frame,
    },

    /// 對一次 [`super::ClientMessage::Commit`] 的答覆：緩衝區裡原樣上屏的文本（拼音字母 / 英文模式下敲的字母）；
    /// 沒在組句時為 `None`。Server 側組句已清空，DLL 收到後把文本落進文件並收起組句。
    Committed {
        /// 會話標識。
        session: SessionId,

        /// 要原樣上屏的文本。
        text: Option<String>,
    },

    /// 不由按鍵觸發的重繪（雲聯想補詞、本地整句模型重排到達），或滑鼠點候選視窗上屏後的新狀態。
    Update {
        /// 會話標識。
        session: SessionId,

        /// 要重繪的狀態。
        frame: Frame,

        /// 滑鼠點候選視窗上屏的文本（v8 起），DLL 寫進文件；老 DLL 讀不到這個欄位，Server 不對它們收點選。
        #[serde(default)]
        commit: Option<String>,
    },

    /// 對一次 [`super::ClientMessage::SyncMode`] 的答覆：當前的全域性中英模式，
    /// 外加當前的按鍵行為設定（每一拍都帶，DLL 那邊熱載入就靠它）。
    ModeSync {
        /// 會話標識。
        session: SessionId,

        /// 全域性模式：`Some(true)` 英文、`Some(false)` 中文，DLL 與自己不同就跟上；`None` 不動（老 Server 沒有待切換時）。
        english: Option<bool>,

        /// 按鍵行為設定；老 DLL 不認識這個欄位，讀到時忽略（serde 預設忽略多餘欄位）。
        #[serde(default)]
        input: InputSettings,

        /// 右鍵選單打勾用的開關狀態（v7 起）。
        #[serde(default)]
        indicator: IndicatorState,
    },

    /// 收到「翻譯選中文字」快捷鍵：請 DLL 在讀編輯會話裡取當前選區，用
    /// [`super::ClientMessage::Selection`] 回。這是對觸發快捷鍵那次 [`super::ClientMessage::Key`] 的應答
    /// （替代常規 [`Self::KeyResult`]）；隨後 DLL 發來的 `Selection` 才引出翻譯候選幀。
    RequestSelection {
        /// 會話標識。
        session: SessionId,

        /// 請求標識，回時帶上。
        request: u64,
    },
}
