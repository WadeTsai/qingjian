mod appearance;
mod apps;
mod aux_code;
mod candidate_renderer;
mod dictionaries;
mod font_size;
mod general;
mod key_combo;
mod layout_mode;
mod log_level;
mod model;
mod modifiers;
mod preedit_mode;
mod scheme;
mod shift_letter;
mod shortcut;
mod status_bar;
mod switch_key;
mod update;

use std::path::Path;

use qingjian_core::FuzzyRules;
use qingjian_predict::PredictConfig;
use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::error::ConfigError;

pub use appearance::Appearance;
pub use apps::{
    AppsConfig, DEFAULT_ENGLISH_CANDIDATES_ON, DEFAULT_ENGLISH_CANDIDATES_ON_LINUX,
    DEFAULT_ENGLISH_CANDIDATES_ON_MACOS, DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS,
};
pub use aux_code::AuxCodeConfig;
pub use candidate_renderer::CandidateRenderer;
pub use dictionaries::{DEFAULT_DOMAINS, DictionariesConfig};
pub use font_size::FontSize;
pub use general::{
    DEFAULT_PAGE_KEYS, DEFAULT_THEME, GeneralConfig, LEARNING_LANGUAGE_OFF, MAX_PAGE_SIZE,
    PAGE_KEY_OPTIONS,
};
pub use key_combo::KeyCombo;
pub use layout_mode::LayoutMode;
pub use log_level::LogLevel;
pub use model::LocalModelConfig;
pub use modifiers::Modifiers;
pub use preedit_mode::PreeditMode;
pub use scheme::{Scheme, scheme_label, scheme_name};
pub use shift_letter::ShiftLetter;
pub use shortcut::ShortcutConfig;
pub use status_bar::StatusBarConfig;
pub use switch_key::{SwitchKey, SwitchKeys};
pub use update::{UpdateChannel, UpdateConfig};

/// 使用者配置檔案（TOML）。所有平臺同一份格式，預設值全部在各分節的 `Default` 裡。
///
/// 配置檔案是唯一事實源：選單、設定視窗、手改檔案三個入口都只寫這個檔案，再由殼熱載入。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 常規：學習語言、每頁候選數、翻頁鍵、外觀。
    pub general: GeneralConfig,

    /// 自定義短語；儲存和讀取均檢查位置衝突。
    #[serde(deserialize_with = "deserialize_phrases")]
    pub custom_phrases: Vec<qingjian_core::CustomPhrase>,

    /// 快捷鍵：字首模式鍵（表示式 / 問字）與上屏譯詞的修飾鍵組合。
    pub shortcut: ShortcutConfig,

    /// 模糊音開關。
    pub fuzzy: FuzzyRules,

    /// 附加詞庫開關。
    pub dictionaries: DictionariesConfig,

    /// 輔碼碼錶開關。
    pub aux_code: AuxCodeConfig,

    /// 按應用改行為（哪些應用裡英文模式不給候選）。
    pub apps: AppsConfig,

    /// 雲聯想。
    pub predict: PredictConfig,

    /// 懸浮狀態條（桌面上常駐、可拖動的中 / 英浮窗）。
    pub status_bar: StatusBarConfig,

    /// 本地整句模型。
    pub model: LocalModelConfig,

    /// 檢查更新。
    pub update: UpdateConfig,
}

fn deserialize_phrases<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<qingjian_core::CustomPhrase>, D::Error> {
    let phrases = Vec::<qingjian_core::CustomPhrase>::deserialize(deserializer)?;
    qingjian_core::custom_phrase::validate_phrases(&phrases).map_err(serde::de::Error::custom)?;
    Ok(phrases)
}

/// 模板的 `[apps]` 一節（Linux）：應用按 fcitx5 認到的名字（X11 是 WM_CLASS，Wayland 是 app_id）。
/// 白名單要與 [`DEFAULT_ENGLISH_CANDIDATES_ON`] 一致，測試 `template_parses_to_defaults` 會核對。
#[cfg(not(any(windows, target_os = "macos")))]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按應用改行為，條目是 fcitx5 認到的應用名（X11 是 WM_CLASS，Wayland 是 app_id；`*` 結尾按字首匹配，不區分大小寫）
# 英文模式下給候選的應用（白名單）：瀏覽器、聊天、辦公與筆記這類寫成段文字的地方。沒列的應用裡英文字母原樣交給應用，
# 免得擋住終端 / 編輯器自己的補全，或吞掉設計、3D 軟體的單鍵快捷鍵。寫成 ["*"] 就處處都給，再用 english_candidates_off 排除個別應用
english_candidates_on = [
  "firefox", "org.mozilla.firefox", "google-chrome", "chromium", "chromium-browser", "microsoft-edge", "brave-browser",
  "thunderbird", "org.mozilla.thunderbird", "libreoffice*", "soffice", "wps", "wpp",
  "gedit", "org.gnome.gedit", "org.gnome.TextEditor", "kate", "org.kde.kate", "kwrite", "mousepad",
  "wechat", "telegram-desktop", "org.telegram.desktop", "discord", "slack", "obsidian", "typora",
]
"#
    };
}

/// 模板的 `[apps]` 一節（macOS）：應用按 bundle identifier 認。白名單要與 [`DEFAULT_ENGLISH_CANDIDATES_ON`] 一致，
/// 測試 `template_parses_to_defaults` 會核對。用宏而不是常量，是因為 `concat!` 只收字面量。
#[cfg(target_os = "macos")]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按應用改行為，條目是 bundle identifier（`*` 結尾按字首匹配）。開著「詳細日誌」時切到一個應用會把它的 bundle identifier 記進日誌
# 英文模式（Caps Lock）下給候選的應用（白名單）：瀏覽器、聊天、辦公與筆記這類寫成段文字的地方。沒列的應用裡英文字母原樣交給應用，
# 免得擋住終端 / 編輯器自己的補全，或吞掉設計、3D 軟體的單鍵快捷鍵。寫成 ["*"] 就處處都給，再用 english_candidates_off 排除個別應用
english_candidates_on = [
  "com.apple.Safari", "com.google.Chrome*", "org.mozilla.firefox", "com.microsoft.edgemac*", "company.thebrowser.Browser", "com.brave.Browser*",
  "com.apple.TextEdit", "com.apple.Notes", "com.apple.mail", "com.apple.MobileSMS", "com.apple.iWork.Pages", "com.apple.iWork.Keynote",
  "com.microsoft.Word", "com.microsoft.Powerpoint", "com.microsoft.Outlook", "com.microsoft.onenote.mac", "com.microsoft.teams2", "com.kingsoft.wpsoffice.mac",
  "com.tencent.xinWeChat", "com.tencent.qq", "com.tencent.WeWorkMac", "com.alibaba.DingTalkMac", "com.electron.lark",
  "ru.keepcoder.Telegram", "com.tdesktop.Telegram", "com.hnc.Discord", "com.tinyspeck.slackmacgap", "md.obsidian", "notion.id", "abnerworks.Typora",
]
"#
    };
}

/// 模板的 `[apps]` 一節（Windows）：應用按宿主程序的 exe 檔名認。白名單要與 [`DEFAULT_ENGLISH_CANDIDATES_ON`] 一致。
#[cfg(windows)]
macro_rules! template_apps {
    () => {
        r#"[apps]
# 按應用改行為，條目是應用程序的 exe 檔名（`*` 結尾按字首匹配）。Server 開著 debug 日誌時每開一個會話會把 exe 名記進日誌
# 英文模式下給候選的應用（白名單）：瀏覽器、聊天、辦公與筆記這類寫成段文字的地方。沒列的應用裡英文字母原樣交給應用，
# 免得擋住終端 / 編輯器自己的補全，或吞掉 Photoshop 這類軟體的單鍵快捷鍵（W / H / V）。寫成 ["*"] 就處處都給，再用 english_candidates_off 排除個別應用
english_candidates_on = [
  "chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "vivaldi.exe", "notepad.exe",
  "WINWORD.EXE", "POWERPNT.EXE", "OUTLOOK.EXE", "ONENOTE.EXE", "ms-teams.exe", "wps.exe", "wpp.exe",
  "Weixin.exe", "WeChat.exe", "QQ.exe", "WXWork.exe", "DingTalk.exe", "Feishu.exe", "Lark.exe",
  "Telegram.exe", "Discord.exe", "slack.exe", "thunderbird.exe", "Obsidian.exe", "Notion.exe", "Typora.exe",
]
"#
    };
}

/// 模板 `[shortcut]` 一節裡的修飾鍵組合（macOS 命名）。預設值兩個平臺一樣，只是寫法與註釋按平臺的鍵名。
#[cfg(not(windows))]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 中 / 英模式切換鍵（Windows 用），可多選：shift 單擊（預設）/ control 單擊 / ctrl+alt+space 組合鍵；[] 不用鍵切換。
# macOS 的切換鍵是 Caps Lock（系統級），本項不生效
switch_mode = ["shift"]
# 數字鍵配這些修飾鍵上屏候選的譯詞：translation 第一個譯詞，translation_second 第二個（候選右側有兩個譯詞時）
# 任意修飾鍵組合（option / shift / control / command 用 + 連），偏好設定裡點按鈕錄製；別用 control+數字（系統切桌面）和 command+數字（應用切標籤頁）
translation = "option"
translation_second = "shift+option"
# 把應用裡選中的文字譯成學習語言（要開著雲服務）：譯文先出現在候選視窗，回車替換選中的文字，Esc 保留原文
# 修飾鍵 + 一個字母或數字，任意組合；避開 ⌘T 這類應用常用鍵
translate_selection = "control+option+t"
# 數字鍵配這些修飾鍵刪掉候選：使用者詞（雲端選過的、自動造的）整個刪掉，詞庫裡的詞清掉對它的學習記錄。組句中要打感嘆號先把詞上屏
delete_candidate = "shift"
"#
    };
}

/// 模板 `[shortcut]` 一節裡的修飾鍵組合（Windows 鍵名：alt / ctrl / win，讀回來與 macOS 的 option / control / command 等價）。
#[cfg(windows)]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 中 / 英模式切換鍵，可多選：shift 單擊（預設，與微軟拼音一致）/ control 單擊 / ctrl+alt+space 組合鍵；[] 不用鍵切換，只剩按鈕。
# 不提供 Ctrl + Space：中文 Windows 把它綁成系統的「輸入法/非輸入法切換」，系統先截走
switch_mode = ["shift"]
# 數字鍵配這些修飾鍵上屏候選的譯詞：translation 第一個譯詞，translation_second 第二個（候選右側有兩個譯詞時）
# 任意修飾鍵組合（alt / shift / ctrl / win 用 + 連）。Alt+數字會被 Windows 當選單快捷鍵截走，預設用 Ctrl；組句時才攔，不打字時照常放行給應用
translation = "ctrl"
translation_second = "shift+ctrl"
# 把應用裡選中的文字譯成學習語言（要開著雲服務）：譯文先出現在候選視窗，回車替換選中的文字，Esc 保留原文
translate_selection = "ctrl+alt+t"
# 數字鍵配這些修飾鍵刪掉候選：使用者詞（雲端選過的、自動造的）整個刪掉，詞庫裡的詞清掉對它的學習記錄。組句中要打感嘆號先把詞上屏
delete_candidate = "shift"
"#
    };
}

/// 首次執行寫出的模板：預設值全部列出並註釋，使用者改一處即可。`[shortcut]` 的修飾鍵與 `[apps]` 分平臺，
/// 見 [`template_shortcut_keys!`] / [`template_apps!`]。
pub const TEMPLATE: &str = concat!(
    r#"# 青簡輸入法配置。儲存後自動生效；也可以在選單欄的輸入法選單裡改。

[general]
# 學習語言（en 英語 / ja 日語 / es 西班牙語 / off 不顯示譯文）：候選旁顯示哪種語言的譯文，要有對應的釋義表才生效
learning_language = "en"
# 每頁候選數（1–9）
page_size = 9
# 翻頁鍵對：前一個上一頁、後一個下一頁。可選 "[]" 或 ",."；選 ",." 的話組句中敲逗號句號是翻頁而不是上屏加標點
page_keys = "[]"
# 候選視窗外觀：system 跟隨系統 / light 淺色 / dark 深色
appearance = "system"
# 候選視窗主題（主題 id）：qingjian 青簡綠 / system-blue 系統藍 / wechat 微信綠 / sakura 櫻花。只對青簡渲染器生效
theme = "qingjian"
# 候選視窗排布：vertical 豎排 / horizontal 橫排（橫排只給高亮候選顯示譯文）
layout = "vertical"
# 橫排時 ↑ / ↓ 把單行展開成 6 行矩陣並換行（一行一頁候選），← / → 改為在候選之間移動（拼音游標用 ⌥←/→、⌘←/→），
# Esc 第一下先收回單行。預設 false：橫排下 ↑ / ↓ 逐個移動高亮、← / → 移動拼音游標，與以前一樣。只有 macOS 用
horizontal_grid = false
# 候選視窗由誰繪製：qingjian 青簡渲染器（各平臺一致，主題走它）/ system 系統原生繪製（渲染器有問題時的退路）
renderer = "qingjian"
# 候選視窗字型（字族名，如 "LXGW WenKai"）；空為系統字型。只對青簡渲染器生效，沒裝這個字型時自動回到系統字型
font = ""
# 候選字、譯文的字號（點），蓋過主題；0 用主題的。只對青簡渲染器生效
candidate_font_size = 0
annotation_font_size = 0
# 候選視窗的過渡動畫（高亮滑動、主題裡的迴圈動畫）；false 時直接跳到位，與系統「減弱動態效果」一樣
animations = true
# 組句中的拼音顯示在哪：both 行內和候選視窗 / inline 只在行內 / window 只在候選視窗（應用裡不放 marked text）
preedit = "both"
# 英文模式（Caps Lock 亮著）是否給英文候選：Tab 或方向鍵選詞，空格、回車、標點仍原樣上屏敲的字母；false 就是純直通
english_candidates = true

# 繁體輸出模式。開啟後上屏繁體，不影響詞庫和個人詞頻的簡體記錄。
traditional = false
# 中文模式下整段輸入是英文詞時（hello / key）是否讓中文候選排第一、英文詞第二；預設 false：拼音不像話的輸入英文詞排第一
chinese_first = false
# 中文模式下整段是英文詞或英文詞的開頭時給英文候選與補全（hello、compa → company）；false 時中文模式只出中文，英文模式不受影響
english_in_chinese = true
# 候選後面配 emoji（kaixin → 開心 😄）；false 時候選裡只有字詞
emoji = true
# 中文模式下按住 Shift 敲的字母：passthrough 拼音原樣上屏、字母交給應用（預設，與以前一致）/ compose 收進組句
# 緩衝區參與匹配，這樣 Cpan 與 cpan 一樣能出「C盤」。英文模式與英文直輸段（no-Way）不受影響
shift_letter = "passthrough"
# 內建英文模式：開著時單擊切換鍵（[shortcut] switch_mode）或 Caps Lock 亮著進英文模式
# 關掉後青簡保持中文模式，切換鍵與語言欄按鈕都不再切過去；要打英文請用系統快捷鍵（Win+Space）切到別的輸入法。只有 Windows 用，macOS 的中英切換是 Caps Lock
english_mode = true
# 中文模式下（沒在組句時）敲的標點轉全形：, . ? ! : ; ( ) 等，數字後面的 . 保持半形。Windows 上懸浮狀態條的「，。」格可以點著切；macOS 在偏好設定中選擇預設中文標點模式
full_width_punctuation = true
# 英文模式下的同一件事，中英各記一份，狀態條切的是當前模式那份；只有 Windows 用
english_full_width_punctuation = false
# 組句中敲會轉全形的標點（, . ? ! 等，翻頁鍵除外）先把高亮候選上屏、再補該標點（nihao, 一氣打完「你好，」）；
# 預設 false：標點進英文直輸段，與以前一致
punct_commits = false
# 輔碼觸發鍵：拼音打完之後敲它進輔碼態，之後敲的字母按碼錶縮小候選範圍；預設是分號
# 單個可見字元，字母、數字與翻頁鍵不能當觸發鍵；微軟 / 搜狗雙拼裡分號先當 ing 的韻母鍵
aux_code_key = ";"
# 候選上是否顯示碼（方括號緊跟在候選詞後面，如「鶴[rbm] crane」）。預設關
aux_code_show = false
# 碼段刪空後是否留在輔碼狀態：true 刪空後 ; 仍在、候選全部回來，再按一次退格才退出輔碼；false 刪空即回拼音狀態
aux_code_keep_empty = true
# 拼音方案：留空或 pinyin 為全拼 / xiaohe 小鶴雙拼 / ziranma 自然碼 / microsoft 微軟雙拼 / sogou 搜狗雙拼 / abc 智能ABC / xiaolang 小浪雙拼 / shoudao 首道雙拼 /
# zhuyin 注音 / none 關（只用形碼，見下面的 wubi）。
# 雙拼與注音下 v / u / i 都是按鍵，表示式模式沒有入口，問字只能靠 question_mark 開啟後用 ? 進；微軟、搜狗方案的 ; 鍵是 ing
scheme = ""
# 雙拼方案下 preedit 顯示原始按鍵（如 ljse）還是展開成全拼（lan'se）；預設 false（展開成全拼）
shuangpin_raw_preedit = false
# 注音鍵盤排列：留空或 dachen 為標準（大千）/ eten 倚天。方案選注音時與 Windows「中文(台灣)」下的「青簡注音」都按它解鍵
zhuyin_layout = ""
# 五筆（86 版形碼）：留空為關，wubi86 為開。**與上面的拼音方案同時開著就是混輸**——
# 兩邊都出候選，編碼打全的五筆詞在前、其次拼音（打不出的字直接打拼音）；候選旁的譯文、生詞記錄與學習照常。
# 只用五筆的話把 scheme 寫成 none；第 5 個字母起五筆已經查不到東西，自動只剩拼音。
wubi = ""
# 日誌級別：info 預設 / debug 詳細（會記錄敲的拼音與上屏的文字，配合作者排查問題時再開）。日誌在 ~/Library/Logs/Qingjian/
log_level = "info"
# 輸入日誌：每次上屏記一行到資料目錄的 input-log.jsonl（敲的鍵、看到的候選、選了什麼），只寫在這臺電腦上，不上傳；
# 用來離線評測排序和訓練個人模型。false 不記；「高階」頁可以清空
input_log = true
# 學習輸入習慣：按你的選擇調整候選順序、記新詞與敲錯糾正。false 不再學，已學的仍參與排序；學習資料在資料目錄，刪掉檔案即清空
learning = true
# 把系統設定「鍵盤 → 文本替換」裡的條目當自定義短語：輸入碼（小寫字母）敲全後短語出現在該碼最靠前的空位；只有 macOS 用
system_text_replacements = true

# 自定義短語示例：取消下面各行註釋後啟用；同碼同位置不能重複。
# [[custom_phrases]]
# code = "ww"       # 輸入碼：1–32 個小寫英文字母
# text = "；"       # 原樣上屏的文本，可包含空格與換行
# position = 1      # 固定候選位置：1–9
# enabled = true    # 是否啟用；停用仍保留位置

[shortcut]
# 字首模式鍵，只能是 v / u / i 之一且互不相同（這三個字母不是任何拼音音節的開頭）
# 表示式模式：v1+2 出 3，v123 出中文數字
expression = "v"
# 問字模式：usangemu 問「三個木」（雲端答），u4e00 出碼點對應的字元（本地答）
question = "u"
# 沒在組句時敲 ? 是否也進問字模式（中英文模式都行，後面跟字母才是問題，跟別的鍵還原成問號）；false 的話問號就是問號
question_mark = false
"#,
    template_shortcut_keys!(),
    r#"
[fuzzy]
# 模糊音：開了之後敲 zi 也出 zhi 的字、敲 lan 也出 nan 的字。預設全關，按需開啟。
z_zh = false
c_ch = false
s_sh = false
n_l = false
f_h = false
l_r = false
an_ang = false
en_eng = false
in_ing = false

"#,
    template_apps!(),
    r#"
[dictionaries]
# 隨包的領域詞庫（法律 / 醫學 / 地名 / 成語 / 詩詞 / IT / 財經 / 飲食 / 動物 / 汽車 / 歷史人物），列在這裡的才載入；
# 名字是檔名：animals automotive finance food historical_figures idioms it_computing law medicine places poetry_lines。
# 偏好設定「詞庫」頁可以勾選
domains = ["idioms"]
# 自己匯入的詞庫：放在配置同目錄 dicts/ 下的 .qj 檔案都會載入，這裡列出要關掉的（檔名，不含副檔名）
disabled = []

[aux_code]
# 輔碼總開關：false 時整條輔碼線關（; 完全保持原生行為，候選也不掛碼）；
# true 且有可用碼錶（隨包或 codes/ 下有 .qj）才生效
enabled = false
# 輔碼碼錶：放在配置同目錄 codes/ 下的 .qj 檔案都會載入，這裡列出要關掉的（檔名，不含副檔名）
# 隨包的筆畫表也可以在這裡關掉；碼錶由「輔碼」設定頁匯入，或放好檔案後在這裡管
disabled = []

[model]
# 本地整句模型：隨包的小模型在本機給整句候選重新排序，全程離線；停頓後幾十毫秒生效。關掉只用詞庫統計
enabled = true

[predict]
# 雲聯想：把游標附近的文本發到下面的介面，讓模型補全整句 / 聯想下文。預設關閉。
# 開啟後選單欄的「中 / 英」旁會帶一個雲朵標識；Secure Input（密碼框）裡絕不傳送。
enabled = false
# OpenAI 相容介面地址與模型名（DeepSeek 預設值）
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
# 推理強度（reasoning_effort）：none 關掉模型的思考，聯想要快；留空則不發這個引數
reasoning_effort = "none"
# 金鑰：填在這裡，或留空並設定 api_key_env 指定的環境變數（偏好設定裡填的金鑰寫進配置同目錄的 .env）
# api_key = ""
api_key_env = "QINGJIAN_API_KEY"
# 單次請求超時（毫秒）、停止敲鍵多久後才發請求（毫秒）
timeout_ms = 5000
debounce_ms = 300
# 游標前 / 後最多發多少個字元——這是發往雲端的上下文上限
lookback = 64
lookahead = 32
# 雲端詞到了補進候選視窗第一頁末尾幾格（比如 2 就是 8、9 兩格），前面的本地候選不動；0 表示不要雲端詞
slots = 2
# 組句中除了詞候選還要不要整句補全（preedit 右側，Tab 接受）
sentence = true

[status_bar]
# 桌面上常駐、可拖動的懸浮狀態條（Windows）：「中 / 英」格點一下切換模式（開著雙拼時還顯示方案名）、「，。」格切全形 / 半形標點、齒輪開啟設定。
# 只在當前輸入法是青簡時顯示；與工作列的中 / 英指示器並存
# 預設關；開著時可以拖到任意位置，拖到哪下次還在哪（拖動結束時把位置寫進下面的 x / y，不用手填）
enabled = false
# 記住的螢幕位置（物理畫素，拖動後自動寫入）；留空則首次出現在螢幕右下角
# x = 0
# y = 0

[update]
# 檢查更新：每天向官網（qingjian.app）讀一次版本索引，有新版在選單與設定的「關於」頁提示；請求不帶任何標識，不自動下載安裝
check = true
# 渠道：stable 只看正式版；beta 還會提示測試版（alpha / beta / rc）
channel = "stable"
"#
);

impl Config {
    /// 儲存自定義短語列表，衝突時不修改檔案。
    pub fn set_custom_phrases(
        path: &Path,
        phrases: &[qingjian_core::CustomPhrase],
    ) -> Result<(), String> {
        qingjian_core::custom_phrase::validate_phrases(phrases)?;
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(e) => return Err(e.to_string()),
        };
        let mut document: DocumentMut = source.parse::<DocumentMut>().map_err(|e| e.to_string())?;
        let old = document
            .get("custom_phrases")
            .and_then(toml_edit::Item::as_array_of_tables)
            .cloned()
            .unwrap_or_default();
        let mut used = std::collections::BTreeSet::new();
        // 先按輸入碼和位置匹配，重排或刪除時註釋跟隨原規則。
        let mut matches: Vec<_> = phrases
            .iter()
            .map(|p| {
                let found = old
                    .iter()
                    .enumerate()
                    .find(|(_, t)| {
                        t.get("code").and_then(toml_edit::Item::as_str) == Some(p.code.as_str())
                            && t.get("position").and_then(toml_edit::Item::as_integer)
                                == Some(p.position as i64)
                    })
                    .map(|(i, _)| i);
                if let Some(i) = found {
                    used.insert(i);
                }
                found
            })
            .collect();
        // 等長列表中修改了輸入碼或位置的條目，沿用其未被佔用的原表。
        if old.len() == phrases.len() {
            for (i, matched) in matches.iter_mut().enumerate() {
                if matched.is_none() && used.insert(i) {
                    *matched = Some(i);
                }
            }
        }
        let mut tables = toml_edit::ArrayOfTables::new();
        for (p, matched) in phrases.iter().zip(matches) {
            let mut t = matched
                .and_then(|i| old.get(i))
                .cloned()
                .unwrap_or_default();
            for (key, mut value) in [
                ("code", toml_edit::Value::from(p.code.as_str())),
                ("text", toml_edit::Value::from(p.text.as_str())),
                ("position", toml_edit::Value::from(p.position as i64)),
                ("enabled", toml_edit::Value::from(p.enabled)),
            ] {
                if let Some(previous) = t.get(key).and_then(toml_edit::Item::as_value) {
                    if previous.to_string() == value.to_string() {
                        continue;
                    }
                    *value.decor_mut() = previous.decor().clone();
                }
                t[key] = toml_edit::Item::Value(value);
            }
            // 序列化按文件位置排序；統一錨點後，同組條目使用本次列表順序。
            t.set_position(old.iter().filter_map(toml_edit::Table::position).min());
            tables.push(t);
        }
        document["custom_phrases"] = toml_edit::Item::ArrayOfTables(tables);
        write_file(path, &document.to_string()).map_err(|e| e.to_string())
    }

    /// 讀配置。檔案不存在按預設值；存在但解析失敗報錯，不要靜默吞掉使用者的筆誤。
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let config: Self = toml::from_str(&source).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        // 配置或環境變數裡的金鑰登記給日誌掩碼；各程序都從這裡載入配置，登記在這一處就夠
        if let Some(key) = config.predict.resolve_api_key() {
            crate::logs::secrets::register(&key);
        }
        Ok(config)
    }

    /// 原地改一個布林鍵，見 [`Self::set_value`]。
    pub fn set_bool(path: &Path, section: &str, key: &str, value: bool) -> Result<(), ConfigError> {
        Self::set_value(path, section, key, value)
    }

    /// 原地改一個鍵（`[section] key = value`），其餘內容、註釋與順序原樣保留：
    /// 選單和設定視窗落盤都走這裡。檔案不存在時從模板起步；檔案有語法錯誤就報錯不寫，
    /// 不能替使用者「修復」成丟了註釋的檔案。
    pub fn set_value(
        path: &Path,
        section: &str,
        key: &str,
        value: impl Into<toml_edit::Value>,
    ) -> Result<(), ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut document: DocumentMut = source.parse().map_err(|source| ConfigError::Edit {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        // 分節不存在時先建成標準表，否則 toml_edit 會寫成頂層的行內表 `predict = { enabled = true }`
        if !document.get(section).is_some_and(|item| item.is_table()) {
            document[section] = toml_edit::table();
        }
        // 舊寫法（外觀寫在 theme 裡）要在改鍵之前認出來：寫的正是 theme 時，新值會把它蓋掉
        let legacy = (section == "general" && matches!(key, "appearance" | "theme"))
            .then(|| legacy_appearance(&document))
            .flatten();
        document[section][key] = toml_edit::value(value);
        if let Some(old) = legacy {
            migrate_legacy_theme(&mut document, old);
        }
        // 寫臨時檔案再改名：輸入法程序隨時可能被殺，不能留半個配置檔案
        write_file(path, &document.to_string())
    }

    /// 原地把一個鍵改成字串陣列（`[section] key = ["a", "b"]`），其餘內容、註釋與順序原樣保留。
    /// 設定介面改詞庫列表（`[dictionaries] domains` / `disabled`）走這裡，[`Self::set_value`] 只能寫標量。
    pub fn set_array<S: AsRef<str>>(
        path: &Path,
        section: &str,
        key: &str,
        values: &[S],
    ) -> Result<(), ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut document: DocumentMut = source.parse().map_err(|source| ConfigError::Edit {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        if !document.get(section).is_some_and(|item| item.is_table()) {
            document[section] = toml_edit::table();
        }
        let mut array = toml_edit::Array::new();
        for value in values {
            array.push(value.as_ref());
        }
        document[section][key] = toml_edit::value(array);
        write_file(path, &document.to_string())
    }

    /// 檔案不存在時寫出模板（目錄一併建），返回是否寫了。
    pub fn write_template_if_missing(path: &Path) -> Result<bool, ConfigError> {
        if path.exists() {
            return Ok(false);
        }
        write_file(path, TEMPLATE)?;
        Ok(true)
    }
}

/// 模板裡 `appearance` 那一行上方的註釋，舊配置遷移時照抄（有測試保證與 [`TEMPLATE`] 一致）。
const APPEARANCE_COMMENT: &str = "# 候選視窗外觀：system 跟隨系統 / light 淺色 / dark 深色\n";

/// 模板裡 `theme` 那一行上方的註釋，同上。
const THEME_COMMENT: &str = "# 候選視窗主題（主題 id）：qingjian 青簡綠 / system-blue 系統藍 / wechat 微信綠 / sakura 櫻花。只對青簡渲染器生效\n";

/// 舊寫法裡 `theme` 寫的外觀；新寫法（或沒寫）為 `None`。
fn legacy_appearance(document: &DocumentMut) -> Option<Appearance> {
    let value = document.get("general")?.get("theme")?.as_str()?;
    Appearance::from_key(value.trim())
}

/// 寫 `appearance` 或 `theme` 時順手遷移舊寫法（2026-09-18 之前外觀寫在 `theme` 裡，舊值是 `old`）：
/// 外觀補寫到 `appearance`（寫的是 `theme` 時它還沒有，不補就丟了），`theme` 還是外觀詞就換成內建主題，
/// 兩行註釋換成模板裡的，免得手改配置的人看到「外觀」註釋下面是 `theme`。遷過之後 `theme` 不再是外觀詞，不會再遷。
fn migrate_legacy_theme(document: &mut DocumentMut, old: Appearance) {
    let Some(general) = document
        .get_mut("general")
        .and_then(toml_edit::Item::as_table_mut)
    else {
        return;
    };
    if !general.contains_key("appearance") {
        general["appearance"] = toml_edit::value(old.key());
    }
    let still_legacy = general
        .get("theme")
        .and_then(toml_edit::Item::as_str)
        .is_some_and(|value| Appearance::from_key(value.trim()).is_some());
    if still_legacy {
        general["theme"] = toml_edit::value(DEFAULT_THEME);
    }
    if let Some(mut key) = general.key_mut("theme") {
        key.leaf_decor_mut().set_prefix(THEME_COMMENT);
    }
    if let Some(mut key) = general.key_mut("appearance") {
        key.leaf_decor_mut().set_prefix(APPEARANCE_COMMENT);
    }
}

/// 原子寫配置檔案；資料目錄還沒有就先建（新賬戶第一次開啟設定時輸入法可能還沒跑過）。
fn write_file(path: &Path, text: &str) -> Result<(), ConfigError> {
    let write = || {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        qingjian_core::storage::write_atomic_str(path, text)
    };
    write().map_err(|source| ConfigError::Write {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests;
