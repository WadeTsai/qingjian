use qingjian_core::{ShuangpinScheme, ZhuyinLayout};
use serde::{Deserialize, Serialize};

use super::scheme::{Scheme, scheme_label};
use super::{
    Appearance, CandidateRenderer, FontSize, LayoutMode, LogLevel, PreeditMode, ShiftLetter,
};

/// 每頁最多幾個候選：數字鍵只有 1–9。
pub const MAX_PAGE_SIZE: usize = 9;

/// 翻頁鍵對的可選值，第一項是預設：第一個鍵向前、第二個向後。
/// 預設不用 `,` `.`：組句中敲逗號句號應該把首選上屏再補一個全形標點（`nihao,zaima` 一氣打完），
/// 拿它們翻頁就得先按空格再敲標點。選 `-` `=` 時組句中的 `-` 是翻頁，不再進英文直輸段（#43）。
pub const PAGE_KEY_OPTIONS: [&str; 3] = ["[]", ",.", "-="];

/// 內建主題的 id。`system` / `light` / `dark` 是外觀的寫法，不能當主題 id（舊配置裡 `theme` 寫的是它們）。
pub const DEFAULT_THEME: &str = "qingjian";

/// 預設翻頁鍵對，與 [`PAGE_KEY_OPTIONS`] 第一項一致。
pub const DEFAULT_PAGE_KEYS: (char, char) = ('[', ']');

/// `[general]` 分節：與具體功能無關的常規項。
/// `learning_language` 寫這個值表示不顯示譯文。
pub const LEARNING_LANGUAGE_OFF: &str = "off";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// 學習語言（ISO 639-1，`en` / `ja` / `es` / `vi`；`off` 不顯示譯文）：候選旁顯示哪種語言的譯文。要有對應的釋義表文件才生效。
    pub learning_language: String,

    /// 每頁候選數，1–9。
    pub page_size: usize,

    /// 翻頁鍵對，兩個字元：前一個上一頁、後一個下一頁。
    pub page_keys: String,

    /// 候選視窗外觀，讀用 [`Self::appearance`]。檔案裡沒寫時為 `None`（欄位級預設），好認出舊寫法：
    /// 2026-09-18 之前外觀寫在 `theme` 裡（`theme = "dark"`）。
    #[serde(default)]
    pub appearance: Option<Appearance>,

    /// 候選視窗主題 id（主題目錄名），讀用 [`Self::theme_id`]。舊寫法裡這一項是外觀，見上。
    pub theme: String,

    /// 候選視窗豎排 / 橫排。
    pub layout: LayoutMode,

    /// 橫排時上 / 下鍵把單行展開成多行矩陣（左 / 右鍵改為移動候選高亮，Esc 第一下先收回）。預設關：橫排下的按鍵與以前一樣。只有 macOS 用。
    pub horizontal_grid: bool,

    /// 候選視窗由青簡渲染器還是系統原生繪製。
    pub renderer: CandidateRenderer,

    /// 候選視窗字型的字族名；空為系統字型。只對青簡渲染器生效，沒裝這個字型時回到系統字型。
    pub font: String,

    /// 候選字字號（點），蓋過主題的；0 用主題的。不限範圍，不是正數的當 0。只對青簡渲染器生效。
    pub candidate_font_size: FontSize,

    /// 譯文字號（點），同上。
    pub annotation_font_size: FontSize,

    /// 候選視窗的過渡與迴圈動畫。關掉與系統「減弱動態效果」一樣：高亮直接跳過去，迴圈動畫停在第一幀。
    pub animations: bool,

    /// 組句中的拼音顯示在行內、候選視窗還是兩處都顯示。
    pub preedit: PreeditMode,

    /// 英文模式（Caps Lock 亮著）是否給英文候選（補全與拼錯糾正）。關掉就是純直通。
    pub english_candidates: bool,

    /// 繁體輸出模式。
    pub traditional: bool,
    /// 中文模式下中英混輸時中文候選總排在英文詞前面。預設關：拼音不像話的輸入（`hello`）英文詞排第一，
    /// 常在中文模式裡打英文詞的人不受影響；想要中文永遠在前的自己開啟。
    pub chinese_first: bool,

    /// 中文模式下整段是英文詞或英文詞的開頭時給英文候選與補全（`hello`、`compa` → company）。關掉中文模式只出中文，英文模式照舊。
    pub english_in_chinese: bool,

    /// 候選後面配 emoji（`kaixin` → 開心 😄）。關掉候選裡就只有字詞。
    pub emoji: bool,

    /// 中文模式下按住 Shift 敲的字母：交給應用（預設）還是收進組句緩衝區參與匹配。
    /// 收進組句才能打出「C盤」這類混雜詞（`Cpan` 與 `cpan` 一樣匹配）。
    pub shift_letter: ShiftLetter,
    /// 內建英文模式：開著時單擊切換鍵（`[shortcut] switch_mode`）或按 Caps Lock 能進英文模式。
    /// 關掉後青簡保持中文模式，切換鍵與語言欄按鈕都不再切過去；要打英文請用系統快捷鍵切到別的輸入法。
    pub english_mode: bool,

    /// 中文模式下不在組句時敲的標點轉成全形（`，。？！` 等，數字後的 `.` 保持半形）。
    /// Windows 懸浮狀態條上可點切換；macOS 在偏好設定中選擇預設模式。
    pub full_width_punctuation: bool,

    /// 英文模式下的同一件事，中英各記一份；預設半形。只有 Windows 用（macOS 英文模式一律半形）。
    pub english_full_width_punctuation: bool,

    /// 組句中敲會轉全形的標點（`,` `.` `?` `!` 等，翻頁鍵除外）先把高亮候選上屏、再補該標點
    ///（`nihao,` 一氣打完「你好，」）。預設關：標點進英文直輸段整段原樣上屏，與以前一致。
    pub punct_commits: bool,

    /// 輔碼觸發鍵：拼音打完之後敲它進輔碼態，預設 `;`。校驗 = 單字元、ASCII 可列印、
    /// 非字母數字、非翻頁鍵（見 [`qingjian_core::is_valid_aux_code_key`]）。
    pub aux_code_key: String,

    /// 候選上是否顯示碼（與譯文拼成一條註記）。預設關：豎排會擠、橫排更難放下。
    pub aux_code_show: bool,

    /// 碼段刪空後是否留在輔碼態：開（預設）時 `;` 仍在、候選全部回來，空碼段再按一次退格才退出；
    /// 關則刪空即回純拼音態。
    pub aux_code_keep_empty: bool,

    /// 拼音側方案：`pinyin`（全拼，預設）/ `xiaohe` / `ziranma` / `microsoft` / `sogou` / `abc` / `xiaolang` / `shoudao` / `zhuyin`
    /// / `none`（關，只用形碼），見 [`Scheme`]。用不認識的寫法時按全拼並警告。
    /// 預設是空串：檔案裡沒寫這一項時要去看舊鍵，見 [`Self::scheme`]。
    pub scheme: String,

    /// 雙拼方案下 preedit 顯示原始按鍵字母（如 `ljse`）還是展開成全拼音節（`lan'se`）。
    /// 預設關（展開成全拼音節）；常在雙拼中打英文詞或需要核對按鍵的人可開啟。
    pub shuangpin_raw_preedit: bool,

    /// 注音鍵盤排列：`dachen`（標準，預設）/ `eten`（倚天），見 [`ZhuyinLayout`]。方案選注音時與 Windows
    /// 「中文(台灣)」下的「青簡注音」都按它解鍵；寫得不認識時按大千並警告。
    pub zhuyin_layout: String,

    /// 形碼側方案：空串為關，`wubi86` 為五筆（86 版）。**與拼音同時開著就是混輸**，見 [`Self::mixed`]。
    pub wubi: String,

    /// 舊鍵（2026-09-16 之前是 `[general] shuangpin`，空串為全拼）：只在 [`Self::scheme`] 裡用來推斷方案，
    /// 不再寫出去；`scheme` 寫了值就不看它。當時 `shuangpin` 與 `zhuyin` 是兩個欄位表達同一個維度。
    pub shuangpin: Option<String>,

    /// 舊鍵（同上的 `[general] zhuyin`）：同上。
    pub zhuyin: Option<bool>,

    /// 日誌級別，預設 info（不含使用者敲的內容）。
    pub log_level: LogLevel,

    /// 輸入日誌：每次上屏記一行到資料目錄的 `input-log.jsonl`（敲的鍵、看到的候選、選了什麼），只寫本機，
    /// 給離線迴歸評測與個人模型用。預設開；關掉就不記，「進階」頁可清空。
    pub input_log: bool,

    /// 學習輸入習慣：按選擇調整候選順序、記新詞與敲錯糾正。關掉後不再記，已學的仍參與排序。
    pub learning: bool,

    /// 把系統的文本替換（macOS「鍵盤 → 文本替換」）並進自定義短語：輸入碼敲全後短語佔該碼最靠前的空位。只有 macOS 用。
    pub system_text_replacements: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            learning_language: "en".to_owned(),
            page_size: MAX_PAGE_SIZE,
            page_keys: PAGE_KEY_OPTIONS[0].to_owned(),
            appearance: Some(Appearance::default()),
            theme: DEFAULT_THEME.to_owned(),
            layout: LayoutMode::default(),
            horizontal_grid: false,
            renderer: CandidateRenderer::default(),
            font: String::new(),
            candidate_font_size: FontSize::default(),
            annotation_font_size: FontSize::default(),
            animations: true,
            preedit: PreeditMode::default(),
            english_candidates: true,
            traditional: false,
            chinese_first: false,
            english_in_chinese: true,
            emoji: true,
            shift_letter: ShiftLetter::default(),
            english_mode: true,
            full_width_punctuation: true,
            english_full_width_punctuation: false,
            punct_commits: false,
            aux_code_key: qingjian_core::DEFAULT_AUX_CODE_KEY.to_string(),
            aux_code_show: false,
            aux_code_keep_empty: true,
            scheme: String::new(),
            shuangpin_raw_preedit: false,
            zhuyin_layout: String::new(),
            wubi: String::new(),
            shuangpin: None,
            zhuyin: None,
            log_level: LogLevel::default(),
            input_log: true,
            learning: true,
            system_text_replacements: true,
        }
    }
}

impl GeneralConfig {
    /// 候選視窗外觀。`appearance` 沒寫時看舊寫法（`theme` 寫的是 system / light / dark），都沒有就跟隨系統。
    pub fn appearance(&self) -> Appearance {
        self.appearance
            .or_else(|| Appearance::from_key(self.theme.trim()))
            .unwrap_or_default()
    }

    /// 候選視窗主題 id。沒寫、空串或舊寫法（外觀詞）都是內建主題。
    pub fn theme_id(&self) -> &str {
        let id = self.theme.trim();
        if id.is_empty() || Appearance::from_key(id).is_some() {
            DEFAULT_THEME
        } else {
            id
        }
    }

    /// 拼音側方案。`scheme` 沒寫時用舊鍵（`shuangpin` / `zhuyin`）推，都沒有就是全拼。
    pub fn scheme(&self) -> Scheme {
        let key = self.scheme.trim();
        if !key.is_empty() {
            return match key.parse() {
                Ok(scheme) => scheme,
                Err(_) => {
                    tracing::warn!(key, "不认识的拼音方案，按全拼");
                    Scheme::Pinyin
                }
            };
        }
        if self.zhuyin == Some(true) {
            tracing::info!("[general] zhuyin 已并入 scheme，可改成 scheme = \"zhuyin\"");
            return Scheme::Zhuyin;
        }
        let Some(legacy) = self
            .shuangpin
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Scheme::Pinyin;
        };
        match legacy.parse::<ShuangpinScheme>() {
            Ok(scheme) => {
                tracing::info!(
                    key = scheme.key(),
                    "[general] shuangpin 已并入 scheme，可改成它"
                );
                Scheme::Shuangpin(scheme)
            }
            Err(_) => {
                tracing::warn!(key = legacy, "不认识的双拼方案，按全拼");
                Scheme::Pinyin
            }
        }
    }

    /// 學習語言關著（`learning_language = "off"`）：候選旁不顯示譯文，生詞標記與釋義兜底也停。
    pub fn learning_language_off(&self) -> bool {
        self.learning_language
            .trim()
            .eq_ignore_ascii_case(LEARNING_LANGUAGE_OFF)
    }

    /// 當前方案是雙拼時是哪一套；不是雙拼時為 `None`。
    pub fn shuangpin(&self) -> Option<ShuangpinScheme> {
        self.scheme().shuangpin()
    }

    /// 當前方案是不是注音。
    pub fn is_zhuyin(&self) -> bool {
        self.scheme() == Scheme::Zhuyin
    }

    /// 注音鍵盤排列；沒寫或空串是大千，認不出來時警告並按大千。
    pub fn zhuyin_layout(&self) -> ZhuyinLayout {
        match self.zhuyin_layout.parse() {
            Ok(layout) => layout,
            Err(_) => {
                tracing::warn!(
                    key = self.zhuyin_layout.trim(),
                    "不认识的注音键盘排列，按大千"
                );
                ZhuyinLayout::Dachen
            }
        }
    }

    /// 形碼側（五筆）開沒開。`wubi` 認 `wubi86`；認不出來時警告並按關。
    /// 舊配置把五筆寫在 `scheme` 裡（`wubi86`），那時等價於「拼音關 + 五筆開」，[`Self::scheme`] 會解成
    /// [`Scheme::Off`]，這裡跟著認下來，免得老配置升級後兩個軸都關著、一個候選都不出。
    pub fn wubi(&self) -> bool {
        let key = self.wubi.trim();
        if key.is_empty() || key == "off" || key == "none" {
            return matches!(self.scheme.trim(), "wubi" | "wubi86");
        }
        if key == "wubi86" {
            return true;
        }
        tracing::warn!(key, "不认识的形码方案，按关");
        false
    }

    /// 拼音與形碼同時開著 = 混輸：兩邊都出候選，編碼打全的形碼詞在前。
    pub fn mixed(&self) -> bool {
        self.scheme().is_on() && self.wubi()
    }

    /// 狀態條上顯示的輸入方案名；見 [`scheme_label`]。
    pub fn scheme_label(&self) -> String {
        scheme_label(self.scheme(), self.wubi())
    }

    /// 輔碼觸發鍵；寫得不對（不是單個合法字元，或撞上當前翻頁鍵 / 拼音隔音符 `'`）時退回預設 `;`。
    pub fn aux_code_key(&self) -> char {
        let mut chars = self.aux_code_key.chars();
        match (chars.next(), chars.next()) {
            (Some(key), None) if qingjian_core::is_valid_aux_code_key(key, self.page_keys()) => key,
            _ => qingjian_core::DEFAULT_AUX_CODE_KEY,
        }
    }

    /// 夾到合法範圍的每頁候選數。
    pub fn page_size(&self) -> usize {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }

    /// 翻頁鍵對；寫得不對（不是兩個不同的 ASCII 可見字元）時退回預設。
    pub fn page_keys(&self) -> (char, char) {
        let mut chars = self.page_keys.chars();
        match (chars.next(), chars.next(), chars.next()) {
            (Some(previous), Some(next), None)
                if previous != next
                    && previous.is_ascii_graphic()
                    && next.is_ascii_graphic()
                    && !previous.is_ascii_alphanumeric()
                    && !next.is_ascii_alphanumeric() =>
            {
                (previous, next)
            }
            _ => DEFAULT_PAGE_KEYS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_grid_is_off_unless_switched_on() {
        assert!(!GeneralConfig::default().horizontal_grid);
        let general: GeneralConfig = toml::from_str("layout = \"horizontal\"\n").unwrap();
        assert!(!general.horizontal_grid);
        let general: GeneralConfig = toml::from_str("horizontal_grid = true\n").unwrap();
        assert!(general.horizontal_grid);
    }

    #[test]
    fn page_size_and_keys_are_sanitized() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.page_size(), 9);
        assert_eq!(general.page_keys(), ('[', ']'));
        general.page_size = 0;
        general.page_keys = ",.".to_owned();
        assert_eq!(general.page_size(), 1);
        assert_eq!(general.page_keys(), (',', '.'));
        general.page_size = 42;
        general.page_keys = "ab".to_owned();
        assert_eq!(general.page_size(), 9);
        assert_eq!(general.page_keys(), ('[', ']'));
        general.page_keys = ",,".to_owned();
        assert_eq!(general.page_keys(), ('[', ']'));
    }

    /// 組句中標點先上屏候選預設關（`[general] punct_commits`），保持標點進英文直輸段的老行為。
    #[test]
    fn punct_commits_defaults_off() {
        assert!(!GeneralConfig::default().punct_commits);
        let general: GeneralConfig = toml::from_str("punct_commits = true\n").unwrap();
        assert!(general.punct_commits);
    }

    #[test]
    fn aux_code_key_falls_back_to_the_default() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.aux_code_key(), ';');
        assert!(!general.aux_code_show);
        general.aux_code_key = "/".to_owned();
        assert_eq!(general.aux_code_key(), '/');
        for bad in ["", "ab", "a", "1", "[", "中"] {
            general.aux_code_key = bad.to_owned();
            assert_eq!(general.aux_code_key(), ';', "{bad}");
        }
    }

    /// 碼刪空保持輔碼態的開關預設開（`[general] aux_code_keep_empty`）。
    #[test]
    fn aux_code_keep_empty_defaults_on() {
        assert!(GeneralConfig::default().aux_code_keep_empty);
    }

    /// 注音鍵盤排列（`[general] zhuyin_layout`）預設大千，認不出來的寫法也退回大千。
    #[test]
    fn zhuyin_layout_defaults_to_dachen_and_unknown_names_fall_back() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.zhuyin_layout(), ZhuyinLayout::Dachen);
        general.zhuyin_layout = "eten".to_owned();
        assert_eq!(general.zhuyin_layout(), ZhuyinLayout::Eten);
        general.zhuyin_layout = " standard ".to_owned();
        assert_eq!(general.zhuyin_layout(), ZhuyinLayout::Dachen);
        general.zhuyin_layout = "hsu".to_owned();
        assert_eq!(general.zhuyin_layout(), ZhuyinLayout::Dachen);
        let general: GeneralConfig = toml::from_str("zhuyin_layout = \"eten\"\n").unwrap();
        assert_eq!(general.zhuyin_layout(), ZhuyinLayout::Eten);
    }

    #[test]
    fn scheme_defaults_to_pinyin_and_unknown_names_fall_back() {
        let mut general = GeneralConfig::default();
        assert_eq!(general.scheme(), Scheme::Pinyin);
        general.scheme = "xiaohe".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Xiaohe));
        general.scheme = " Sogou ".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Sogou));
        general.scheme = "xiaolang".to_owned();
        assert_eq!(general.shuangpin(), Some(ShuangpinScheme::Xiaolang));
        general.scheme = "none".to_owned();
        assert_eq!(general.scheme(), Scheme::Off);
        assert!(!general.scheme().is_on());
        general.scheme = "flypy".to_owned();
        assert_eq!(general.scheme(), Scheme::Pinyin);
    }

    #[test]
    fn the_two_axes_are_independent_and_their_combination_is_mixed_input() {
        let mut general = GeneralConfig::default();
        // 預設：全拼，不開形碼
        assert_eq!(general.scheme(), Scheme::Pinyin);
        assert!(!general.wubi());
        assert!(!general.mixed());

        // 雙拼 alone
        general.scheme = "xiaohe".to_owned();
        assert!(!general.mixed());
        // 五筆 alone：拼音側關掉
        general.scheme = "none".to_owned();
        general.wubi = "wubi86".to_owned();
        assert!(general.wubi());
        assert!(!general.mixed());
        // 組合：兩邊都開 = 混輸
        general.scheme = "xiaohe".to_owned();
        assert!(general.mixed());
        // 形碼寫得不認識：按關，且不影響拼音側
        general.wubi = "wubi98".to_owned();
        assert!(!general.wubi());
        assert_eq!(general.scheme(), Scheme::Shuangpin(ShuangpinScheme::Xiaohe));
    }

    #[test]
    fn the_old_wubi_scheme_spelling_still_turns_wubi_on() {
        // 2026-09-16 之前五筆是 `scheme = "wubi86"`（單選）。升級後兩個軸都得跟上，
        // 否則老配置會變成「拼音關 + 形碼關」，一個字都打不出來。
        let general = GeneralConfig {
            scheme: "wubi86".to_owned(),
            ..GeneralConfig::default()
        };
        assert_eq!(general.scheme(), Scheme::Off);
        assert!(general.wubi());
        assert!(!general.mixed());
    }

    #[test]
    fn files_written_before_the_scheme_key_keep_their_scheme() {
        let parse = |text: &str| toml::from_str::<GeneralConfig>(text).unwrap().scheme();
        assert_eq!(
            parse("shuangpin = \"xiaohe\"\n"),
            Scheme::Shuangpin(ShuangpinScheme::Xiaohe)
        );
        assert_eq!(parse("zhuyin = true\n"), Scheme::Zhuyin);
        assert_eq!(parse("shuangpin = \"\"\nzhuyin = false\n"), Scheme::Pinyin);
        assert_eq!(parse(""), Scheme::Pinyin);
        // 新鍵寫了就以它為準
        assert_eq!(
            parse("scheme = \"pinyin\"\nshuangpin = \"xiaohe\"\n"),
            Scheme::Pinyin
        );
    }

    #[test]
    fn legacy_shuangpin_and_zhuyin_keys_are_read_but_scheme_wins() {
        let mut general = GeneralConfig {
            scheme: String::new(),
            ..GeneralConfig::default()
        };
        assert_eq!(general.scheme(), Scheme::Pinyin);
        general.shuangpin = Some("xiaohe".to_owned());
        assert_eq!(general.scheme(), Scheme::Shuangpin(ShuangpinScheme::Xiaohe));
        general.zhuyin = Some(true);
        assert_eq!(general.scheme(), Scheme::Zhuyin);
        assert!(general.is_zhuyin());
        // 舊配置裡兩個都寫是不合法的，新鍵寫了就不看它們
        general.scheme = "pinyin".to_owned();
        assert_eq!(general.scheme(), Scheme::Pinyin);
        assert!(!general.is_zhuyin());
    }
}
