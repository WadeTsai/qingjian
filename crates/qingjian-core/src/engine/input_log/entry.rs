use serde::{Deserialize, Serialize};

use super::InputSource;

/// 一次上屏。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitEntry {
    /// 本程序內單調遞增的序號，撤銷條目用它指回來。
    pub id: u64,

    /// 查詢時整段作用域的原始鍵（回放評測就是把它重新餵給引擎）；雙拼下是雙拼鍵，糾錯時是敲錯的原串。
    #[serde(default)]
    pub scope: String,

    /// 這次上屏消耗掉的那部分原始鍵（`scope` 的字首）。
    pub keys: String,

    /// 查詢時整段作用域的切分（`'` 連線；糾錯生效時是糾正後的拼音）。
    pub pinyin: String,

    /// 拼寫糾錯是否生效。
    pub corrected: bool,

    /// 上屏的文字。
    pub text: String,

    /// 來源。
    pub source: InputSource,

    /// 選的是查詢時候選列表裡的第幾個（從 0 數）；不在列表裡（晚到的雲端詞、原樣上屏）為 `None`。
    pub index: Option<usize>,

    /// 查詢時排在前面的幾個候選文本，用來離線算首選命中率。
    pub top: Vec<String>,

    /// 雙拼方案的鍵（`xiaohe`），全拼為空。
    pub scheme: String,

    /// 是否在英文模式。
    pub english: bool,

    /// 查詢時的候選順序經過了神經重排（本地整句模型的分已經進了排序）。
    #[serde(default)]
    pub rescored: bool,

    /// 正在輸入的應用（macOS bundle identifier / Windows exe 名）；殼沒給為 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,

    /// 這段組句裡翻了幾頁候選（翻回來也算一次）。
    #[serde(default)]
    pub pages: u32,

    /// 這段組句從第一鍵到這次上屏的毫秒數；一段拼音分幾次上屏時每次都從第一鍵算。
    #[serde(default)]
    pub ms: u64,
}

/// 日誌裡的一條。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum InputLogEntry {
    /// 上屏。
    Commit(CommitEntry),

    /// 使用者把上一次上屏的詞整個退格刪掉又對同一段拼音選了別的：那次選錯了。
    Retract {
        /// 被撤銷的那條上屏的 `id`。
        of: u64,

        /// 被撤銷的文字。
        text: String,

        /// 改選的文字。
        chosen: String,
    },

    /// 輸入法啟動或日誌開啟時記一次：之後的條目是哪個版本、什麼配置下記的。
    Session {
        /// 日誌格式版本（見 `docs/plan/model-eval.md`）。
        v: u32,

        /// 輸入法版本號。
        version: String,

        /// 平臺（`macos` / `windows` / `cli`）。
        platform: String,

        /// 本地整句模型（神經重打分）開沒開。
        model: bool,

        /// 雙拼方案的鍵，全拼為空。
        scheme: String,
    },

    /// 使用者重打了鍵：組句裡退格之後上屏的鍵串與第一次退格前不同，或上屏後刪掉重打了相近但不同的鍵。
    /// 敲錯糾正的真實樣本。
    Retype {
        /// 退格前的鍵串。
        before: String,

        /// 最終上屏的鍵串。
        after: String,

        /// 對應的上屏 `id`（組句內是這次上屏，跨上屏是被刪掉的那次）。
        of: u64,
    },

    /// 組句之外直接交給應用的字元（標點、回車、空格、英文模式的字母），連續的攢成一條：文本流的分隔與段落邊界。
    Passthrough {
        /// 攢下的字元。
        text: String,
    },

    /// 雲端聯想的結果到達並展示了：給過使用者什麼，緊接著的 `commit` 說明接沒接受。
    Prediction {
        /// 請求時的作用域（原始鍵）。
        scope: String,

        /// 雲端詞文本。
        words: Vec<String>,

        /// 整句補全。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sentence: Option<String>,
    },

    /// 上文鏈斷了（切換應用、失焦、清空）：文本流的段落邊界。
    Break {
        /// 斷開之後所在的應用；殼沒給為 `None`。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        app: Option<String>,
    },
}

/// 當前日誌格式版本。
pub const INPUT_LOG_VERSION: u32 = 1;
