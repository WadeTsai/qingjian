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
# 按应用改行为，条目是 fcitx5 认到的应用名（X11 是 WM_CLASS，Wayland 是 app_id；`*` 结尾按前缀匹配，不区分大小写）
# 英文模式下给候选的应用（白名单）：浏览器、聊天、办公与笔记这类写成段文字的地方。没列的应用里英文字母原样交给应用，
# 免得挡住终端 / 编辑器自己的补全，或吞掉设计、3D 软件的单键快捷键。写成 ["*"] 就处处都给，再用 english_candidates_off 排除个别应用
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
# 按应用改行为，条目是 bundle identifier（`*` 结尾按前缀匹配）。开着「详细日志」时切到一个应用会把它的 bundle identifier 记进日志
# 英文模式（Caps Lock）下给候选的应用（白名单）：浏览器、聊天、办公与笔记这类写成段文字的地方。没列的应用里英文字母原样交给应用，
# 免得挡住终端 / 编辑器自己的补全，或吞掉设计、3D 软件的单键快捷键。写成 ["*"] 就处处都给，再用 english_candidates_off 排除个别应用
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
# 按应用改行为，条目是应用进程的 exe 文件名（`*` 结尾按前缀匹配）。Server 开着 debug 日志时每开一个会话会把 exe 名记进日志
# 英文模式下给候选的应用（白名单）：浏览器、聊天、办公与笔记这类写成段文字的地方。没列的应用里英文字母原样交给应用，
# 免得挡住终端 / 编辑器自己的补全，或吞掉 Photoshop 这类软件的单键快捷键（W / H / V）。写成 ["*"] 就处处都给，再用 english_candidates_off 排除个别应用
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
        r#"# 中 / 英模式切换键（Windows 用），可多选：shift 单击（預設）/ control 单击 / ctrl+alt+space 组合键；[] 不用键切换。
# macOS 的切换键是 Caps Lock（系统级），本项不生效
switch_mode = ["shift"]
# 数字键配这些修饰键上屏候选的译词：translation 第一个译词，translation_second 第二个（候选右侧有两个译词时）
# 任意修饰键组合（option / shift / control / command 用 + 连），偏好设置里点按钮录制；别用 control+数字（系统切桌面）和 command+数字（应用切标签页）
translation = "option"
translation_second = "shift+option"
# 把应用里选中的文字译成学习语言（要开着云服务）：译文先出现在候选窗口，回车替换选中的文字，Esc 保留原文
# 修饰键 + 一个字母或数字，任意组合；避开 ⌘T 这类应用常用键
translate_selection = "control+option+t"
# 数字键配这些修饰键删掉候选：用户词（云端选过的、自动造的）整个删掉，词库里的词清掉对它的学习记录。组句中要打感叹号先把词上屏
delete_candidate = "shift"
"#
    };
}

/// 模板 `[shortcut]` 一節裡的修飾鍵組合（Windows 鍵名：alt / ctrl / win，讀回來與 macOS 的 option / control / command 等價）。
#[cfg(windows)]
macro_rules! template_shortcut_keys {
    () => {
        r#"# 中 / 英模式切换键，可多选：shift 单击（預設，与微软拼音一致）/ control 单击 / ctrl+alt+space 组合键；[] 不用键切换，只剩按钮。
# 不提供 Ctrl + Space：中文 Windows 把它绑成系统的「输入法/非输入法切换」，系统先截走
switch_mode = ["shift"]
# 数字键配这些修饰键上屏候选的译词：translation 第一个译词，translation_second 第二个（候选右侧有两个译词时）
# 任意修饰键组合（alt / shift / ctrl / win 用 + 连）。Alt+数字会被 Windows 当菜单快捷键截走，預設用 Ctrl；组句时才拦，不打字时照常放行给应用
translation = "ctrl"
translation_second = "shift+ctrl"
# 把应用里选中的文字译成学习语言（要开着云服务）：译文先出现在候选窗口，回车替换选中的文字，Esc 保留原文
translate_selection = "ctrl+alt+t"
# 数字键配这些修饰键删掉候选：用户词（云端选过的、自动造的）整个删掉，词库里的词清掉对它的学习记录。组句中要打感叹号先把词上屏
delete_candidate = "shift"
"#
    };
}

/// 首次執行寫出的模板：預設值全部列出並註釋，使用者改一處即可。`[shortcut]` 的修飾鍵與 `[apps]` 分平臺，
/// 見 [`template_shortcut_keys!`] / [`template_apps!`]。
pub const TEMPLATE: &str = concat!(
    r#"# 青简输入法配置。保存后自动生效；也可以在菜单栏的输入法菜单里改。

[general]
# 学习语言（en 英语 / ja 日语 / es 西班牙语 / off 不显示译文）：候选旁显示哪种语言的译文，要有对应的释义表才生效
learning_language = "en"
# 每页候选数（1–9）
page_size = 9
# 翻页键对：前一个上一页、后一个下一页。可选 "[]" 或 ",."；选 ",." 的话组句中敲逗号句号是翻页而不是上屏加标点
page_keys = "[]"
# 候选窗口外观：system 跟随系统 / light 浅色 / dark 深色
appearance = "system"
# 候选窗口主题（主题 id）：qingjian 青简绿 / system-blue 系统蓝 / wechat 微信绿 / sakura 樱花。只对青简渲染器生效
theme = "qingjian"
# 候选窗口排布：vertical 竖排 / horizontal 横排（横排只给高亮候选显示译文）
layout = "vertical"
# 横排时 ↑ / ↓ 把单行展开成 6 行矩阵并换行（一行一页候选），← / → 改为在候选之间移动（拼音光标用 ⌥←/→、⌘←/→），
# Esc 第一下先收回单行。預設 false：横排下 ↑ / ↓ 逐个移动高亮、← / → 移动拼音光标，与以前一样。只有 macOS 用
horizontal_grid = false
# 候选窗口由谁绘制：qingjian 青简渲染器（各平台一致，主题走它）/ system 系统原生绘制（渲染器有问题时的退路）
renderer = "qingjian"
# 候选窗口字体（字族名，如 "LXGW WenKai"）；空为系统字体。只对青简渲染器生效，没装这个字体时自动回到系统字体
font = ""
# 候选字、译文的字号（点），盖过主题；0 用主题的。只对青简渲染器生效
candidate_font_size = 0
annotation_font_size = 0
# 候选窗口的过渡动画（高亮滑动、主题里的循环动画）；false 时直接跳到位，与系统「减弱动态效果」一样
animations = true
# 组句中的拼音显示在哪：both 行内和候选窗口 / inline 只在行内 / window 只在候选窗口（应用里不放 marked text）
preedit = "both"
# 英文模式（Caps Lock 亮着）是否给英文候选：Tab 或方向键选词，空格、回车、标点仍原样上屏敲的字母；false 就是纯直通
english_candidates = true

# 繁体输出模式。开启后上屏繁体，不影响词库和个人词频的简体记录。
traditional = false
# 中文模式下整段输入是英文词时（hello / key）是否让中文候选排第一、英文词第二；預設 false：拼音不像话的输入英文词排第一
chinese_first = false
# 中文模式下整段是英文词或英文词的开头时给英文候选与补全（hello、compa → company）；false 时中文模式只出中文，英文模式不受影响
english_in_chinese = true
# 候选后面配 emoji（kaixin → 开心 😄）；false 时候选里只有字词
emoji = true
# 中文模式下按住 Shift 敲的字母：passthrough 拼音原样上屏、字母交给应用（預設，与以前一致）/ compose 收进组句
# 缓冲区参与匹配，这样 Cpan 与 cpan 一样能出「C盘」。英文模式与英文直输段（no-Way）不受影响
shift_letter = "passthrough"
# 内置英文模式：开着时单击切换键（[shortcut] switch_mode）或 Caps Lock 亮着进英文模式
# 关掉后青简保持中文模式，切换键与语言栏按钮都不再切过去；要打英文请用系统快捷键（Win+Space）切到别的输入法。只有 Windows 用，macOS 的中英切换是 Caps Lock
english_mode = true
# 中文模式下（没在组句时）敲的标点转全角：, . ? ! : ; ( ) 等，数字后面的 . 保持半角。Windows 上悬浮状态条的「，。」格可以点着切；macOS 在偏好设置中选择默认中文标点模式
full_width_punctuation = true
# 英文模式下的同一件事，中英各记一份，状态条切的是当前模式那份；只有 Windows 用
english_full_width_punctuation = false
# 组句中敲会转全角的标点（, . ? ! 等，翻页键除外）先把高亮候选上屏、再补该标点（nihao, 一气打完「你好，」）；
# 預設 false：标点进英文直输段，与以前一致
punct_commits = false
# 辅码触发键：拼音打完之后敲它进辅码态，之后敲的字母按码表缩小候选范围；預設是分号
# 单个可见字符，字母、数字与翻页键不能当触发键；微软 / 搜狗双拼里分号先当 ing 的韵母键
aux_code_key = ";"
# 候选上是否显示码（方括号紧跟在候选词后面，如「鹤[rbm] crane」）。預設关
aux_code_show = false
# 码段删空后是否留在辅码状态：true 删空后 ; 仍在、候选全部回来，再按一次退格才退出辅码；false 删空即回拼音状态
aux_code_keep_empty = true
# 拼音方案：留空或 pinyin 为全拼 / xiaohe 小鹤双拼 / ziranma 自然码 / microsoft 微软双拼 / sogou 搜狗双拼 / abc 智能ABC / xiaolang 小浪双拼 / shoudao 首道双拼 /
# zhuyin 大千注音 / none 关（只用形码，见下面的 wubi）。
# 双拼与注音下 v / u / i 都是按键，表达式模式没有入口，问字只能靠 question_mark 打开后用 ? 进；微软、搜狗方案的 ; 键是 ing
scheme = ""
# 双拼方案下 preedit 显示原始按键（如 ljse）还是展开成全拼（lan'se）；預設 false（展开成全拼）
shuangpin_raw_preedit = false
# 五笔（86 版形码）：留空为关，wubi86 为开。**与上面的拼音方案同时开着就是混输**——
# 两边都出候选，编码打全的五笔词在前、其次拼音（打不出的字直接打拼音）；候选旁的译文、生词记录与学习照常。
# 只用五笔的话把 scheme 写成 none；第 5 个字母起五笔已经查不到东西，自动只剩拼音。
wubi = ""
# 日志级别：info 預設 / debug 详细（会记录敲的拼音与上屏的文字，配合作者排查问题时再开）。日志在 ~/Library/Logs/Qingjian/
log_level = "info"
# 输入日志：每次上屏记一行到数据目录的 input-log.jsonl（敲的键、看到的候选、选了什么），只写在这台电脑上，不上传；
# 用来离线评测排序和训练个人模型。false 不记；「高级」页可以清空
input_log = true
# 学习输入习惯：按你的选择调整候选顺序、记新词与敲错纠正。false 不再学，已学的仍参与排序；学习数据在数据目录，删掉文件即清空
learning = true
# 把系统设置「键盘 → 文本替换」里的条目当自定义短语：输入码（小写字母）敲全后短语出现在该码最靠前的空位；只有 macOS 用
system_text_replacements = true

# 自定义短语示例：取消下面各行注释后启用；同码同位置不能重复。
# [[custom_phrases]]
# code = "ww"       # 输入码：1–32 个小写英文字母
# text = "；"       # 原样上屏的文本，可包含空格与换行
# position = 1      # 固定候选位置：1–9
# enabled = true    # 是否启用；停用仍保留位置

[shortcut]
# 前缀模式键，只能是 v / u / i 之一且互不相同（这三个字母不是任何拼音音节的开头）
# 表达式模式：v1+2 出 3，v123 出中文数字
expression = "v"
# 问字模式：usangemu 问「三个木」（云端答），u4e00 出码点对应的字符（本地答）
question = "u"
# 没在组句时敲 ? 是否也进问字模式（中英文模式都行，后面跟字母才是问题，跟别的键还原成问号）；false 的话问号就是问号
question_mark = false
"#,
    template_shortcut_keys!(),
    r#"
[fuzzy]
# 模糊音：开了之后敲 zi 也出 zhi 的字、敲 lan 也出 nan 的字。默认全关，按需打开。
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
# 随包的领域词库（法律 / 医学 / 地名 / 成语 / 诗词 / IT / 财经 / 饮食 / 动物 / 汽车 / 历史人物），列在这里的才加载；
# 名字是文件名：animals automotive finance food historical_figures idioms it_computing law medicine places poetry_lines。
# 偏好设置「词库」页可以勾选
domains = ["idioms"]
# 自己导入的词库：放在配置同目录 dicts/ 下的 .qj 文件都会加载，这里列出要关掉的（文件名，不含扩展名）
disabled = []

[aux_code]
# 辅码总开关：false 时整条辅码线关（; 完全保持原生行为，候选也不挂码）；
# true 且有可用码表（随包或 codes/ 下有 .qj）才生效
enabled = false
# 辅码码表：放在配置同目录 codes/ 下的 .qj 文件都会加载，这里列出要关掉的（文件名，不含扩展名）
# 随包的笔画表也可以在这里关掉；码表由「辅码」设置页导入，或放好文件后在这里管
disabled = []

[model]
# 本地整句模型：随包的小模型在本机给整句候选重新排序，全程离线；停顿后几十毫秒生效。关掉只用词库统计
enabled = true

[predict]
# 云联想：把光标附近的文本发到下面的接口，让模型补全整句 / 联想下文。默认关闭。
# 开启后菜单栏的「中 / 英」旁会带一个云朵标识；Secure Input（密码框）里绝不发送。
enabled = false
# OpenAI 兼容接口地址与模型名（DeepSeek 默认值）
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
# 推理强度（reasoning_effort）：none 关掉模型的思考，联想要快；留空则不发这个参数
reasoning_effort = "none"
# 密钥：填在这里，或留空并设置 api_key_env 指定的环境变量（偏好设置里填的密钥写进配置同目录的 .env）
# api_key = ""
api_key_env = "QINGJIAN_API_KEY"
# 单次请求超时（毫秒）、停止敲键多久后才发请求（毫秒）
timeout_ms = 5000
debounce_ms = 300
# 光标前 / 后最多发多少个字符——这是发往云端的上下文上限
lookback = 64
lookahead = 32
# 云端词到了补进候选窗口第一页末尾几格（比如 2 就是 8、9 两格），前面的本地候选不动；0 表示不要云端词
slots = 2
# 组句中除了词候选还要不要整句补全（preedit 右侧，Tab 接受）
sentence = true

[status_bar]
# 桌面上常驻、可拖动的悬浮状态条（Windows）：「中 / 英」格点一下切换模式（开着双拼时还显示方案名）、「，。」格切全角 / 半角标点、齿轮打开设置。
# 只在当前输入法是青简时显示；与任务栏的中 / 英指示器并存
# 默认关；开着时可以拖到任意位置，拖到哪下次还在哪（拖动结束时把位置写进下面的 x / y，不用手填）
enabled = false
# 记住的屏幕位置（物理像素，拖动后自动写入）；留空则首次出现在屏幕右下角
# x = 0
# y = 0

[update]
# 检查更新：每天向官网（qingjian.app）读一次版本索引，有新版在菜单与设置的「关于」页提示；请求不带任何标识，不自动下载安装
check = true
# 渠道：stable 只看正式版；beta 还会提示测试版（alpha / beta / rc）
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
const APPEARANCE_COMMENT: &str = "# 候选窗口外观：system 跟随系统 / light 浅色 / dark 深色\n";

/// 模板裡 `theme` 那一行上方的註釋，同上。
const THEME_COMMENT: &str = "# 候选窗口主题（主题 id）：qingjian 青简绿 / system-blue 系统蓝 / wechat 微信绿 / sakura 樱花。只对青简渲染器生效\n";

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
