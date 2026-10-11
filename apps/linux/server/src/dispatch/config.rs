//! Linux 首版按鍵與候選配置。
use qingjian_platform::protocol::KeyModifiers;
use qingjian_platform::{Appearance, AppsConfig, Config, LayoutMode, PreeditMode};

/// Router 要用的配置項，與 macOS 殼的 `Host` 欄位對齊。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterConfig {
    /// 每頁候選數（`[general] page_size`）。
    pub page_size: usize,

    /// 應用與視窗拼音顯示位置。
    pub preedit: PreeditMode,

    /// 雲端候選在第一頁預留的格數（`[predict] slots`）。
    pub cloud_slots: usize,

    /// 候選排布（`[general] layout`）。
    pub layout: LayoutMode,

    /// 候選視窗外觀（`[general] theme`）。
    pub appearance: Appearance,

    /// 翻頁鍵對（`[general] page_keys`，上一頁 / 下一頁）。
    pub page_keys: (char, char),

    /// 英文模式給不給英文候選（`[general] english_candidates`）。
    pub english_candidates: bool,

    /// Shift 大寫字母是否加入中文組句。
    pub shift_letter_compose: bool,

    /// 中文模式下不在組句時的標點轉全形（`[general] full_width_punctuation`）；狀態條可切。
    pub full_width: bool,

    /// 英文模式的那一份（`[general] english_full_width_punctuation`）。
    pub english_full_width: bool,

    /// 組句中敲會轉全形的標點（`,` `.` `?` `!` 等，翻頁鍵除外）先把高亮候選上屏、再補該標點
    ///（`nihao,` 一氣打完「你好，」）的開關（`[general] punct_commits`），預設關。
    pub punct_commits: bool,

    /// 按應用的設定（`[apps]`），按宿主 exe 名認。
    pub apps: AppsConfig,

    /// 上屏第一 / 第二個譯詞的修飾鍵（`[shortcut] translation` / `translation_second`）。
    pub translation_keys: (KeyModifiers, KeyModifiers),

    /// 刪候選的修飾鍵（`[shortcut] delete_candidate`）。
    pub delete_keys: KeyModifiers,
}

impl RouterConfig {
    /// 全域性開關開著，且應用在 `[apps] english_candidates_on` 白名單裡；沒報應用名按不給。
    pub fn english_candidates_in(&self, app: Option<&str>) -> bool {
        self.english_candidates && self.apps.english_candidates_on(app)
    }
}

impl From<&Config> for RouterConfig {
    fn from(config: &Config) -> Self {
        Self {
            page_size: config.general.page_size(),
            preedit: config.general.preedit,
            cloud_slots: 0,
            layout: config.general.layout,
            appearance: config.general.appearance(),
            page_keys: config.general.page_keys(),
            english_candidates: config.general.english_candidates,
            shift_letter_compose: config.general.shift_letter.compose(),
            full_width: config.general.full_width_punctuation,
            english_full_width: config.general.english_full_width_punctuation,
            punct_commits: config.general.punct_commits,
            apps: config.apps.clone(),
            translation_keys: {
                let (first, second) = config.shortcut.translation_keys();
                (first.into(), second.into())
            },
            delete_keys: config.shortcut.delete_keys().into(),
        }
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}
