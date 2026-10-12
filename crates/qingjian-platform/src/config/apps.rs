use serde::{Deserialize, Serialize};

/// 預設給英文候選的應用（macOS，按 bundle identifier）：瀏覽器、聊天、辦公與筆記這類寫成段文字的地方。
/// 白名單而不是黑名單：設計、3D、CAD、影片剪輯、遊戲都靠單鍵快捷鍵（Photoshop 的 W / H / V），
/// 英文候選會把字母收進組句，這類應用列不完。`*` 結尾是字首匹配。
pub const DEFAULT_ENGLISH_CANDIDATES_ON_MACOS: &[&str] = &[
    "com.apple.Safari",
    "com.google.Chrome*",
    "org.mozilla.firefox",
    "com.microsoft.edgemac*",
    "company.thebrowser.Browser", // Arc
    "com.brave.Browser*",
    "com.apple.TextEdit",
    "com.apple.Notes",
    "com.apple.mail",
    "com.apple.MobileSMS",
    "com.apple.iWork.Pages",
    "com.apple.iWork.Keynote",
    "com.microsoft.Word",
    "com.microsoft.Powerpoint",
    "com.microsoft.Outlook",
    "com.microsoft.onenote.mac",
    "com.microsoft.teams2",
    "com.kingsoft.wpsoffice.mac",
    "com.tencent.xinWeChat",
    "com.tencent.qq",
    "com.tencent.WeWorkMac",
    "com.alibaba.DingTalkMac",
    "com.electron.lark", // 飛書
    "ru.keepcoder.Telegram",
    "com.tdesktop.Telegram",
    "com.hnc.Discord",
    "com.tinyspeck.slackmacgap",
    "md.obsidian",
    "notion.id",
    "abnerworks.Typora",
];

/// 預設給英文候選的應用（Windows，按宿主程序的 exe 檔名）。理由同 [`DEFAULT_ENGLISH_CANDIDATES_ON_MACOS`]。
pub const DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS: &[&str] = &[
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "brave.exe",
    "opera.exe",
    "vivaldi.exe",
    "notepad.exe",
    "WINWORD.EXE",
    "POWERPNT.EXE",
    "OUTLOOK.EXE",
    "ONENOTE.EXE",
    "ms-teams.exe",
    "wps.exe",
    "wpp.exe",
    "Weixin.exe", // 微信 4.x
    "WeChat.exe",
    "QQ.exe",
    "WXWork.exe", // 企業微信
    "DingTalk.exe",
    "Feishu.exe",
    "Lark.exe",
    "Telegram.exe",
    "Discord.exe",
    "slack.exe",
    "thunderbird.exe",
    "Obsidian.exe",
    "Notion.exe",
    "Typora.exe",
];

/// 預設給英文候選的應用（Linux，按 fcitx5 的 program 名：X11 是 WM_CLASS，Wayland 是 app_id）。
/// 理由同 [`DEFAULT_ENGLISH_CANDIDATES_ON_MACOS`]；匹配大小寫不敏感，所以大小寫變體不用重複列。
pub const DEFAULT_ENGLISH_CANDIDATES_ON_LINUX: &[&str] = &[
    "firefox",
    "org.mozilla.firefox",
    "google-chrome",
    "chromium",
    "chromium-browser",
    "microsoft-edge",
    "brave-browser",
    "thunderbird",
    "org.mozilla.thunderbird",
    "libreoffice*",
    "soffice",
    "wps",
    "wpp",
    "gedit",
    "org.gnome.gedit",
    "org.gnome.TextEditor",
    "kate",
    "org.kde.kate",
    "kwrite",
    "mousepad",
    "wechat",
    "telegram-desktop",
    "org.telegram.desktop",
    "discord",
    "slack",
    "obsidian",
    "typora",
];

/// 本平臺的預設白名單：macOS 上是 bundle identifier，Windows 上是 exe 檔名，Linux 上是 fcitx5 的 program 名。
#[cfg(windows)]
pub const DEFAULT_ENGLISH_CANDIDATES_ON: &[&str] = DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS;

/// 本平臺的預設白名單：macOS 上是 bundle identifier，Windows 上是 exe 檔名，Linux 上是 fcitx5 的 program 名。
#[cfg(target_os = "macos")]
pub const DEFAULT_ENGLISH_CANDIDATES_ON: &[&str] = DEFAULT_ENGLISH_CANDIDATES_ON_MACOS;

/// 本平臺的預設白名單：macOS 上是 bundle identifier，Windows 上是 exe 檔名，Linux 上是 fcitx5 的 program 名。
#[cfg(not(any(windows, target_os = "macos")))]
pub const DEFAULT_ENGLISH_CANDIDATES_ON: &[&str] = DEFAULT_ENGLISH_CANDIDATES_ON_LINUX;

/// 配置檔案 `[apps]` 分節：按應用改行為。應用的標識 macOS 上是 bundle identifier，Windows 上是宿主程序的 exe 檔名，
/// Linux 上是 fcitx5 認到的應用名。
///
/// 現在只管英文模式給不給候選：先看白名單 `english_candidates_on`，再用 `english_candidates_off` 排除。
/// 全域性開關 `[general] english_candidates` 關著時兩份名單都不起作用。以後按應用定 preedit 模式等也放這裡。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppsConfig {
    /// 英文模式下給候選的應用，`*` 結尾按字首匹配，單寫 `"*"` 就是處處都給。
    pub english_candidates_on: Vec<String>,

    /// 從白名單裡排除的應用（配合 `"*"` 用：處處都給、只有這幾個不給）。預設空。
    pub english_candidates_off: Vec<String>,
}

impl Default for AppsConfig {
    fn default() -> Self {
        Self::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON)
    }
}

impl AppsConfig {
    /// 用給定白名單構造（預設名單分平臺，測試裡要指定哪一份）。
    pub fn with_english_candidates_on(apps: &[&str]) -> Self {
        Self {
            english_candidates_on: apps.iter().map(|s| (*s).to_owned()).collect(),
            english_candidates_off: Vec::new(),
        }
    }

    /// 這個應用裡英文模式給不給候選。`app` 不認識（應用沒報）時只有 `"*"` 給：認不出來的地方寧可原樣交給應用。
    pub fn english_candidates_on(&self, app: Option<&str>) -> bool {
        let Some(app) = app else {
            return self.english_candidates_everywhere();
        };
        self.english_candidates_on
            .iter()
            .any(|pattern| matches_app(pattern, app))
            && !self
                .english_candidates_off
                .iter()
                .any(|pattern| matches_app(pattern, app))
    }

    /// 白名單是不是「處處都給」（`"*"`）：偏好設定裡「只在常用寫字應用裡給候選」的勾選框據此顯示。
    pub fn english_candidates_everywhere(&self) -> bool {
        self.english_candidates_on
            .iter()
            .any(|pattern| pattern.trim() == "*")
    }
}

/// `pattern` 是完整的應用標識，或 `*` 結尾的字首。不區分大小寫（bundle identifier 與 Windows 檔名本身都不區分）。
fn matches_app(pattern: &str, app: &str) -> bool {
    let pattern = pattern.trim();
    match pattern.strip_suffix('*') {
        Some(prefix) => {
            // 應用名不保證是 ASCII（Windows 的 exe 檔名、Linux 的 app_id / WM_CLASS 都可以帶非 ASCII 字元）：
            // 按位元組切片會切在字元中間 panic，`get` 切不到就是不匹配。
            app.get(..prefix.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
        }
        None => pattern.eq_ignore_ascii_case(app),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_list_covers_writing_apps_with_prefix_patterns() {
        let apps = AppsConfig::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON_MACOS);
        assert!(apps.english_candidates_on(Some("com.apple.TextEdit")));
        assert!(apps.english_candidates_on(Some("com.google.Chrome.canary")));
        assert!(apps.english_candidates_on(Some("COM.TENCENT.XINWECHAT")));
        assert!(!apps.english_candidates_on(Some("com.adobe.Photoshop")));
        assert!(!apps.english_candidates_on(Some("com.apple.Terminal")));
        assert!(!apps.english_candidates_on(Some("")));
        assert!(!apps.english_candidates_on(None));
    }

    #[test]
    fn windows_list_matches_exe_names() {
        let apps = AppsConfig::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS);
        assert!(apps.english_candidates_on(Some("Weixin.exe")));
        assert!(apps.english_candidates_on(Some("winword.exe")));
        assert!(!apps.english_candidates_on(Some("Photoshop.exe")));
        assert!(!apps.english_candidates_on(Some("SLDWORKS.exe")));
        assert!(!apps.english_candidates_on(Some("Code.exe")));
        assert!(!apps.english_candidates_on(Some("chrome")));
    }

    #[test]
    fn linux_defaults_match_program_names() {
        let apps = AppsConfig::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON_LINUX);
        assert!(apps.english_candidates_on(Some("firefox")));
        assert!(
            apps.english_candidates_on(Some("ORG.KDE.Kate")),
            "匹配大小寫不敏感"
        );
        assert!(
            apps.english_candidates_on(Some("libreoffice-writer")),
            "* 字首匹配"
        );
        assert!(!apps.english_candidates_on(Some("konsole")));
        assert!(!apps.english_candidates_on(Some("blender")));
    }

    #[test]
    fn default_list_follows_the_platform() {
        let apps = AppsConfig::default();
        assert_eq!(
            apps.english_candidates_on(Some("chrome.exe")),
            cfg!(windows),
            "Windows 預設名單按 exe 名"
        );
        assert_eq!(
            apps.english_candidates_on(Some("com.apple.Safari")),
            cfg!(target_os = "macos"),
            "macOS 預設名單按 bundle identifier"
        );
        assert_eq!(
            apps.english_candidates_on(Some("kate")),
            cfg!(not(any(windows, target_os = "macos"))),
            "Linux 預設名單按 fcitx5 的 program 名"
        );
        assert!(!apps.english_candidates_everywhere());
    }

    #[test]
    fn star_means_everywhere_minus_the_off_list() {
        let apps: AppsConfig = toml::from_str(
            r#"english_candidates_on = ["*"]
english_candidates_off = ["Photoshop.exe"]"#,
        )
        .unwrap();
        assert!(apps.english_candidates_everywhere());
        assert!(apps.english_candidates_on(Some("anything.exe")));
        assert!(!apps.english_candidates_on(Some("photoshop.exe")));
        assert!(apps.english_candidates_on(None), "認不出應用也給");
    }

    #[test]
    fn empty_list_gives_candidates_nowhere() {
        let apps: AppsConfig = toml::from_str("english_candidates_on = []").unwrap();
        assert!(!apps.english_candidates_on(Some("com.apple.TextEdit")));
    }

    #[test]
    fn prefix_pattern_needs_the_whole_prefix() {
        assert!(matches_app("com.jetbrains.*", "com.jetbrains.goland"));
        assert!(!matches_app("com.jetbrains.*", "com.jetbrain"));
        assert!(matches_app("*", "anything"));
    }

    #[test]
    fn prefix_patterns_survive_multibyte_app_names() {
        // 名單裡有 `*` 字首項時，多位元組應用名的第 prefix.len() 位元組可能落在字元中間：
        // 必須判為不匹配，不許 panic。
        let apps = AppsConfig::with_english_candidates_on(&["jetbrains-*", "code"]);
        assert!(!apps.english_candidates_on(Some("日本語入力テスト")));
        assert!(!apps.english_candidates_on(Some("abc日本語")));
        assert!(!apps.english_candidates_on(Some("日本語")));
        assert!(
            apps.english_candidates_on(Some("jetbrains-idea")),
            "既有字首匹配不受影響"
        );
    }
}
