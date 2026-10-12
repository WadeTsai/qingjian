use qingjian_platform::protocol::KeyModifiers;
use qingjian_platform::{
    Appearance, AppsConfig, CandidateRenderer, Config, FontSize, KeyCombo, LayoutMode, PreeditMode,
    Scheme, SwitchKeys,
};

use super::RenderSettings;

/// Windows 版只支援繁體中文：青簡只登記在系統的「中文(台灣)」語言下，不管配置檔案裡 `[general] traditional`
/// 寫的是什麼，一律按開著算。啟動讀配置與熱加載都先過這一步，之後 Router 再從它建 [`RouterConfig`]。
/// 設定介面也不再給這個開關；配置檔案裡的值只給 Linux / macOS 用。
pub fn enforce_traditional(config: &mut Config) {
    config.general.traditional = true;
}

/// Router 要用的配置項，與 macOS 殼的 `Host` 欄位對齊。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterConfig {
    /// 每頁候選數（`[general] page_size`）。
    pub page_size: usize,

    /// 雲端候選在第一頁預留的格數（`[predict] slots`）。
    pub cloud_slots: usize,

    /// 中文模式下 Shift+字母收進組句緩衝區（`[general] shift_letter = "compose"`）。
    /// 關著（預設）時殼把大寫字母交給應用，與以前一致。
    pub shift_letter_compose: bool,

    /// 候選排布（`[general] layout`）。
    pub layout: LayoutMode,

    /// 候選視窗外觀（`[general] appearance`，相容舊寫法見 [`qingjian_platform::GeneralConfig::appearance`]）。
    pub appearance: Appearance,

    /// 候選視窗 / 狀態條由青簡渲染器還是 GDI 畫（`[general] renderer`）。
    pub renderer: CandidateRenderer,

    /// 候選視窗字型的字族名（`[general] font`），空為系統字型；只對青簡渲染器生效。
    pub font: String,

    /// 候選視窗主題 id（`[general] theme`）；只對青簡渲染器生效。
    pub theme: String,

    /// 候選字、譯文字號（`[general] candidate_font_size` / `annotation_font_size`）。
    pub font_sizes: (FontSize, FontSize),

    /// 過渡動畫（`[general] animations`）。
    pub animations: bool,

    /// 使用者主題目錄的戳，熱載入輪詢時更新；不來自配置檔案，重讀配置時沿用。
    pub themes_stamp: u64,

    /// 拼音顯示位置（`[general] preedit`）。
    pub preedit: PreeditMode,

    /// 翻頁鍵對（`[general] page_keys`，上一頁 / 下一頁）。
    pub page_keys: (char, char),

    /// 英文模式給不給英文候選（`[general] english_candidates`）。
    pub english_candidates: bool,

    /// 內建英文模式總開關（`[general] english_mode`）：關掉後狀態條上的「中 / 英」不再切模式
    /// （切換鍵與語言欄按鈕由 DLL 按同一項攔住，見 `com::service::mode`）。
    pub english_mode: bool,

    /// 中英切換鍵（`[shortcut] switch_mode`）：由 Server 經協議下發給 DLL，由它認鍵。
    pub switch_mode: SwitchKeys,

    /// 中文模式下不在組句時的標點轉全形（`[general] full_width_punctuation`）；狀態條可切。
    pub full_width: bool,

    /// 英文模式的那一份（`[general] english_full_width_punctuation`）。
    pub english_full_width: bool,

    /// 組句中可轉全形的標點先把高亮候選上屏再補標點（`[general] punct_commits`）；
    /// 關掉恢復老行為：標點進英文直輸段。預設開。
    pub punct_commits: bool,

    /// 注音（`[general] scheme = "zhuyin"`）。
    pub zhuyin: bool,

    /// 按應用的設定（`[apps]`），按宿主 exe 名認。
    pub apps: AppsConfig,

    /// 上屏第一 / 第二個譯詞的修飾鍵（`[shortcut] translation` / `translation_second`）。
    pub translation_keys: (KeyModifiers, KeyModifiers),

    /// 刪候選的修飾鍵（`[shortcut] delete_candidate`）。
    pub delete_keys: KeyModifiers,

    /// 「翻譯選中文字」快捷鍵（`[shortcut] translate_selection`）。
    pub translate_selection: KeyCombo,

    /// 懸浮狀態條開關（`[status_bar] enabled`）。
    pub status_enabled: bool,

    /// 狀態條記住的位置（`[status_bar] x` / `y`，內容左上角物理畫素）。
    pub status_pos: Option<(i32, i32)>,

    /// 拼音側方案（`[general] scheme`）。
    pub scheme: Scheme,

    /// 形碼側開沒開（`[general] wubi`）。與拼音同時開著就是混輸。
    pub wubi: bool,

    /// 繁體輸出（`[general] traditional`），給候選視窗主題顯示簡 / 繁。Windows 版經 [`enforce_traditional`] 一律為開。
    pub traditional: bool,

    /// 輔碼觸發鍵（`[general] aux_code_key`，預設 `;`）；非法值退回預設。
    pub aux_code_key: char,

    /// 候選上是否顯示輔碼（`[general] aux_code_show`）。隨幀下發給候選窗。
    pub aux_code_show: bool,
}

impl RouterConfig {
    /// 全域性開關開著，且應用在 `[apps] english_candidates_on` 白名單裡；沒報應用名按不給。
    pub fn english_candidates_in(&self, app: Option<&str>) -> bool {
        self.english_candidates && self.apps.english_candidates_on(app)
    }

    /// 交給 UI 執行緒的畫法。
    pub fn render_settings(&self) -> RenderSettings {
        RenderSettings {
            renderer: self.renderer,
            font: self.font.clone(),
            theme: self.theme.clone(),
            font_sizes: self.font_sizes,
            animations: self.animations,
            themes_stamp: self.themes_stamp,
        }
    }
}

impl From<&Config> for RouterConfig {
    fn from(config: &Config) -> Self {
        Self {
            page_size: config.general.page_size(),
            cloud_slots: config.predict.slots,
            shift_letter_compose: config.general.shift_letter.compose(),
            layout: config.general.layout,
            appearance: config.general.appearance(),
            renderer: config.general.renderer,
            font: config.general.font.trim().to_owned(),
            theme: config.general.theme_id().to_owned(),
            font_sizes: (
                config.general.candidate_font_size,
                config.general.annotation_font_size,
            ),
            animations: config.general.animations,
            themes_stamp: 0,
            preedit: config.general.preedit,
            page_keys: config.general.page_keys(),
            english_candidates: config.general.english_candidates,
            english_mode: config.general.english_mode,
            switch_mode: config.shortcut.switch_mode,
            full_width: config.general.full_width_punctuation,
            english_full_width: config.general.english_full_width_punctuation,
            punct_commits: config.general.punct_commits,
            zhuyin: config.general.is_zhuyin(),
            apps: config.apps.clone(),
            translation_keys: {
                let (first, second) = config.shortcut.translation_keys();
                (first.into(), second.into())
            },
            delete_keys: config.shortcut.delete_keys().into(),
            translate_selection: config.shortcut.translate_selection,
            status_enabled: config.status_bar.enabled,
            status_pos: config.status_bar.x.zip(config.status_bar.y),
            scheme: config.general.scheme(),
            wubi: config.general.wubi(),
            traditional: config.general.traditional,
            aux_code_key: config.general.aux_code_key(),
            aux_code_show: config.general.aux_code_show,
        }
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}

#[cfg(test)]
mod tests {
    use qingjian_platform::Config;

    use super::{RouterConfig, enforce_traditional};

    /// Windows 版不管配置檔案裡寫什麼都出繁體：強制後建出的 RouterConfig 繁體位為開。
    #[test]
    fn enforce_traditional_overrides_the_config_file() {
        let mut config = Config::default();
        assert!(!config.general.traditional);
        enforce_traditional(&mut config);
        assert!(config.general.traditional);
        assert!(RouterConfig::from(&config).traditional);
    }
}
