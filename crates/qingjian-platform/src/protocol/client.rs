use serde::{Deserialize, Serialize};

use super::indicator::IndicatorCommand;
use super::key::KeyEvent;
use super::screen_rect::ScreenRect;
use super::session::SessionId;

/// DLL（客戶端，每個應用程序裡一個）發給 Server 的訊息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// 進入一個 TSF 文件，開一個會話。
    OpenSession {
        /// 會話標識。
        session: SessionId,

        /// 宿主應用的 exe 檔名（`Code.exe`），DLL 載入在應用程序裡直接取；取不到為 `None`。
        /// Server 據此查 `[apps]` 分節的按應用設定（對應 macOS 的 bundle identifier）。
        #[serde(default)]
        app: Option<String>,

        /// DLL 編譯時的 [`super::PROTOCOL_VERSION`]。升級安裝後舊 DLL 仍留在沒重啟的應用裡，
        /// Server 對不上只記警告照常服務；老 DLL 不帶此欄位，讀成 0。
        #[serde(default)]
        protocol: u32,
    },

    /// 一次按鍵，等 Server 回 [`super::ServerMessage::KeyResult`]。
    Key {
        /// 會話標識。
        session: SessionId,

        /// 按鍵內容。
        event: KeyEvent,
    },

    /// 焦點離開 / 文件要求結束組句：Server 把緩衝區裡的內容原樣交出並清空，回 [`super::ServerMessage::Committed`]。
    Commit {
        /// 會話標識。
        session: SessionId,
    },

    /// 組句期間 DLL 定時輪詢：取雲聯想的非同步結果（雲端候選 / 整句補全）。Server 拉一次
    /// `poll_prediction`，把最新組句狀態經 [`super::ServerMessage::Update`] 回給 DLL。傳輸仍是一問一答，
    /// 雲結果靠 DLL 側定時器拉取，不需要 Server 主動推。
    Poll {
        /// 會話標識。
        session: SessionId,
    },

    /// 組句起始時 DLL 主動送來的應用游標前文，給本地整句模型當前文（對應 macOS 殼在組句第一鍵讀 `surrounding_text`）。
    /// 在起組句的那次編輯會話裡順手讀，不另開會話、不回話；密碼框 / 讀不到時不發，Server 退回本會話歷史。
    Surrounding {
        /// 會話標識。
        session: SessionId,

        /// 游標前最多 64 字。
        text: String,
    },

    /// 輸入框私密與否變了（DLL 起組句時按輸入範圍判：`IS_PRIVATE` / 密碼 / PIN 類算私密，瀏覽器無痕視窗就是它）。
    /// Server 讓 Engine 進 / 出私密：不學習、不記輸入日誌、不發雲端；前文 DLL 側就不讀。只在與上次報的不同時發，不回話。
    /// 真正的密碼框（`KEYBOARD_DISABLED` compartment）DLL 直接放行所有鍵、不組句，到不了這裡。
    Privacy {
        /// 會話標識。
        session: SessionId,

        /// 現在是私密輸入。
        private: bool,
    },

    /// 回應 [`super::ServerMessage::RequestSelection`]：應用當前選中的文字（供「翻譯選中文字」）。
    /// DLL 在讀編輯會話裡用 `GetSelection` + `GetText` 取；沒有選區 / 讀不到時 `text` 為空串。
    Selection {
        /// 會話標識。
        session: SessionId,

        /// 請求標識，對上是哪一次 [`super::ServerMessage::RequestSelection`]。
        request: u64,

        /// 選中的文字；沒有選區時為空串。
        text: String,

        /// 選區的螢幕矩形（拿翻譯候選視窗擺在它下方，與組句候選窗一致）；取不到是滑鼠處近似。
        rect: ScreenRect,
    },

    /// 組句更新後，DLL 在編輯會話裡量到組句範圍的螢幕矩形，發來讓 Server 把候選視窗擺到游標下方。
    /// 不等回話：候選視窗由 Server 程序自繪（搬出應用程序，才能蓋過微軟商店 / 工作列搜尋這些高 z-band 宿主）。
    /// 組句結束 / 失焦時 Server 按空幀與 [`Commit`](Self::Commit) 自行收視窗，不必 DLL 再發。
    PositionCandidates {
        /// 會話標識。
        session: SessionId,

        /// 組句範圍的螢幕矩形（拿不到時是滑鼠處的一個近似矩形）。
        rect: ScreenRect,
    },

    /// 組句在 DLL 側結束、而 Server 無從知曉時（應用強行終止組句 `OnCompositionTerminated`、斷連兜底），
    /// 讓 Server 收起候選視窗。Server 按空幀 / [`Commit`](Self::Commit) 能自行收視窗的場合不需要這條。
    /// 不等回話。
    HideCandidates {
        /// 會話標識。
        session: SessionId,
    },

    /// 使用者在這個應用裡切了中英模式：DLL 報給 Server，成為全域性模式（其餘應用下次取模式時跟上，懸浮狀態條同步）。
    /// 不等回話。雙拼方案 Server 從自己的配置裡知道，不必帶。
    ModeChanged {
        /// 會話標識。
        session: SessionId,

        /// `true` 英文模式，`false` 中文模式。
        english: bool,
    },

    /// 取全域性中英模式：啟用、得到焦點時各一次，前臺、沒在組句時再定時問（別的應用或懸浮狀態條可能切過）。
    /// 回 [`super::ServerMessage::ModeSync`]；有 DLL 來取也說明青簡是當前輸入法。
    SyncMode {
        /// 會話標識。
        session: SessionId,
    },

    /// 宿主執行緒把輸入法切成了別的（微軟拼音等）：Server 收起懸浮狀態條。應用退出時不發（那時狀態條該留著），
    /// 所以狀態條的顯隱不跟會話開關走。DLL 在被停用後才收到這個通知，用一條臨時連線發；不等回話。
    ImeSwitched {
        /// 會話標識。
        session: SessionId,
    },

    /// 這個會話所線上程用的是青簡的哪個語言 profile（v9 起）：Windows 上青簡在「中文(簡體)」下登記「青简」，
    /// 「中文(台灣)」下登記「青簡」與「青簡注音」。「中文(台灣)」下的兩個不看 `[general] traditional` 就出繁體
    ///（配置開著時全都出繁體）；「青簡注音」還不看 `[general] scheme`、按大千注音解鍵。
    /// 開會話後不是預設 profile（簡體、按配置方案）才報一次，之後同一執行緒在 profile 間切換再報；
    /// 沒有 profile 概念的平臺（Linux）不發，Server 按預設 profile 算。不等回話。
    Profile {
        /// 會話標識。
        session: SessionId,

        /// 當前是繁體的 profile（「中文(台灣)」下的青簡 / 青簡注音）。
        traditional: bool,

        /// 當前是注音的 profile（「中文(台灣)」下的青簡注音）：拼音側按大千注音，不看配置的方案。
        /// 老 DLL 不帶此欄位，讀成 `false`（同一版本週期內加的欄位，協議號不再升）。
        #[serde(default)]
        zhuyin: bool,
    },

    /// 工作列「中 / 英」圖示右鍵選單裡點了一項（v7 起）。不等回話。
    Indicator {
        /// 會話標識。
        session: SessionId,

        /// 點的是哪一項。
        command: IndicatorCommand,
    },

    /// 關閉會話，釋放 Server 側狀態。
    CloseSession {
        /// 會話標識。
        session: SessionId,
    },
}
