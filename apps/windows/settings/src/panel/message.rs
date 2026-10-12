//! 設定視窗的訊息型別：導航切換與各頁的「改動」，根元件的 `update` 據此落盤。

/// 設定視窗的訊息；「改動」訊息帶控制元件新值，`update` 據此落盤。
#[derive(Clone)]
pub(crate) enum Message {
    /// 導航切換分節（`None` 是取消選中，忽略）。
    Navigate(Option<String>),

    // 通用頁
    LearningLanguage(Option<usize>),
    PageSize(Option<f64>),
    Scheme(Option<usize>),
    /// 注音鍵盤排列（`[general] zhuyin_layout`）。
    ZhuyinLayout(Option<usize>),
    ShuangpinRawPreedit(bool),
    Wubi(bool),
    Emoji(bool),
    EnglishCandidates(bool),
    EnglishInChinese(bool),
    ChineseFirst(bool),
    /// 中文模式下 Shift+字母：交給應用（預設）還是進組句緩衝區。
    ShiftLetter(Option<usize>),
    FullWidthPunctuation(bool),
    EnglishFullWidthPunctuation(bool),
    /// 組句中會轉全形的標點先把高亮候選上屏再補標點。
    PunctCommits(bool),
    /// 開=寫入平臺預設白名單，關=寫 `["*"]`（處處都給）。
    EnglishOnlyInApps(bool),
    /// 勾上 / 去掉一箇中英切換鍵。
    SwitchKey(qingjian_platform::SwitchKey, bool),
    /// 內建英文模式總開關。
    EnglishMode(bool),

    // 候選視窗頁
    Appearance(Option<usize>),
    Theme(Option<usize>),
    Layout(Option<usize>),
    Preedit(Option<usize>),
    Renderer(Option<usize>),
    /// 字型框裡的文字變了：空或正好是某個字族名就落盤。
    FontQuery(String),
    /// 從提示裡選了一個字族。
    Font(String),
    /// 字號框改了；清空為 `None`（用主題的）。
    CandidateFontSize(Option<f64>),
    AnnotationFontSize(Option<f64>),
    Animations(bool),
    ResetFontSizes,
    StatusBar(bool),

    // 雲服務頁
    LocalModel(bool),
    CloudEnabled(bool),
    CloudApiKey(String),
    CloudModel(String),
    CloudBaseUrl(String),
    CloudSlots(Option<f64>),
    CloudSentence(bool),
    TestConnection,
    CloudTestDone(Result<String, String>),

    // 快捷鍵頁
    PageKeys(Option<usize>),
    ModeExpression(Option<usize>),
    ModeQuestion(Option<usize>),
    QuestionMark(bool),
    Translation(Option<usize>),
    TranslationSecond(Option<usize>),
    DeleteCandidate(Option<usize>),
    /// 只換修飾鍵，字母鍵固定用當前的。
    TranslateSelection(Option<usize>),

    // 模糊音頁
    /// 配置鍵 + 新值。
    Fuzzy(&'static str, bool),

    // 詞庫頁
    ToggleDomain(String, bool),
    ToggleUserDict(String, bool),
    /// 挪進 dicts\removed，不真刪。
    RemoveUserDict(String),
    ImportDictionary,

    // 輔碼頁
    /// 輔碼總開關（`[aux_code] enabled`，預設關）。
    AuxCodeEnabled(bool),
    /// 候選上是否顯示碼。
    AuxCodeShow(bool),
    /// 碼刪空後是否留在輔碼態（`[general] aux_code_keep_empty`）。
    AuxCodeKeepEmpty(bool),
    /// 點「錄製」：進入等一個鍵的狀態。
    AuxRecordStart,
    /// 錄製中放棄，保持原值。
    AuxRecordCancel,
    /// 錄製框敲進來的文本，取第一個字元當新觸發鍵。
    AuxRecorded(String),
    /// 碼錶開關（名字，開 / 關），關掉的進 `[aux_code] disabled`。
    ToggleAuxTable(String, bool),
    /// 挪進 codes\removed，不真刪。
    RemoveAuxTable(String),
    /// 開啟檔案選擇器匯入一張碼錶（Rime `.dict.yaml`）。
    ImportCodeTable,

    // 進階頁
    VerboseLog(bool),
    InputLog(bool),
    /// 學習輸入習慣開關。
    Learning(bool),
    OpenConfigFile,
    OpenDataDir,
    OpenLogDir,
    /// 日誌目錄 + config.toml 打成 zip 放桌面。
    ExportLogs,
    ClearInputLog,

    // 關於頁
    OpenWebsite,

    // 關於頁：檢查更新（`UpdateChecked` 的 `None` = 本地開發版沒查）
    UpdateCheck(bool),
    UpdateChannel(Option<usize>),
    CheckUpdateNow,
    UpdateChecked(Option<Result<qingjian_update::UpdateState, String>>),
    OpenDownload,
    OpenRepository,
}
