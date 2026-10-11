//! 「通用」頁：學習語言、每頁候選數、輸入方案、英文模式候選。

use qingjian_platform::{MAX_PAGE_SIZE, Scheme, ShiftLetter, SwitchKey};
use windows_reactor::*;

use crate::panel::controls::{feedback, field, index_of, page};
use crate::panel::{Message, Settings};

/// 學習語言：介面名 + 配置寫法。
pub(crate) const LANGUAGES: [(&str, &str); 5] = [
    ("英語", "en"),
    ("日語", "ja"),
    ("西班牙語", "es"),
    ("越南語", "vi"),
    ("不顯示譯文", "off"),
];

/// 輸入方案：介面名 + 配置寫法，直接照 [`Scheme::ALL`] 建，不另抄一份。
/// 陣列長度取自 `ALL`，以後加方案時這裡陣列對不上就編不過。
pub(crate) const SCHEMES: [(&str, &str); Scheme::ALL.len()] = [
    (Scheme::ALL[0].label(), Scheme::ALL[0].key()),
    (Scheme::ALL[1].label(), Scheme::ALL[1].key()),
    (Scheme::ALL[2].label(), Scheme::ALL[2].key()),
    (Scheme::ALL[3].label(), Scheme::ALL[3].key()),
    (Scheme::ALL[4].label(), Scheme::ALL[4].key()),
    (Scheme::ALL[5].label(), Scheme::ALL[5].key()),
    (Scheme::ALL[6].label(), Scheme::ALL[6].key()),
    (Scheme::ALL[7].label(), Scheme::ALL[7].key()),
    (Scheme::ALL[8].label(), Scheme::ALL[8].key()),
    (Scheme::ALL[9].label(), Scheme::ALL[9].key()),
];

pub(crate) fn string_combo(
    options: &'static [(&str, &str)],
    current: &str,
    callback: Callback<Option<usize>>,
) -> ComboBox {
    ComboBox::new()
        .items_source(options.iter().map(|(label, _)| *label))
        .selected_index(index_of(options, current))
        .on_selection_changed(callback)
}

/// Shift+字母的下拉：選項直接由 [`ShiftLetter::ALL`] 生成，免得再抄一份表（順序要和它一致）。
fn shift_letter_combo(current: ShiftLetter, callback: Callback<Option<usize>>) -> ComboBox {
    ComboBox::new()
        .items_source(ShiftLetter::ALL.iter().map(|mode| mode.label()))
        .selected_index(ShiftLetter::ALL.iter().position(|mode| *mode == current))
        .on_selection_changed(callback)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let english_only_in_apps = !settings.config.apps.english_candidates_everywhere();
    let rows = [
        field(
            "學習語言",
            "候選詞右側顯示哪種語言的譯詞，只列出裝了釋義表的語言；「不顯示譯文」同時關掉生詞標記與釋義兜底。",
            string_combo(
                &LANGUAGES,
                &g.learning_language,
                context.callback(Message::LearningLanguage),
            ),
        ),
        field(
            "每頁候選數",
            "",
            NumberBox::new()
                .minimum(1.0)
                .maximum(MAX_PAGE_SIZE as f64)
                .value(g.page_size as f64)
                .on_value_changed(context.callback(Message::PageSize)),
        ),
        field(
            "拼音方案",
            "全拼、五套雙拼、大千注音，或關（只用下面的五筆）。\
             雙拼下 v、u、i 是音節鍵，表示式與問字模式改用 Shift+V、Shift+U 進（微軟、搜狗方案的 ; 鍵是 ing）；\
             注音下 v、u、i 也是按鍵，只能用 ? 開頭進。",
            string_combo(
                &SCHEMES,
                g.scheme().key(),
                context.callback(Message::Scheme),
            ),
        ),
        field(
            "雙拼在輸入框顯示原始按鍵",
            "勾上後雙拼模式下輸入框（游標處）顯示敲擊的英文字母，回車可直接上屏；候選視窗頂部的拼音行照舊顯示解碼全拼。",
            ToggleSwitch::new()
                .is_on(g.shuangpin_raw_preedit)
                .is_enabled(g.scheme().is_shuangpin())
                .on_toggled(context.callback(Message::ShuangpinRawPreedit)),
        ),
        field(
            "五筆（86 版）",
            "與拼音方案同時開著就是混輸：編碼打全的五筆詞在前，打不出的字直接打拼音。\
             單用五筆請把拼音方案關掉；第 5 個字母起五筆查不到東西，自動只剩拼音。\
             譯詞、生詞記錄與學習照常。",
            ToggleSwitch::new()
                .is_on(g.wubi())
                .on_toggled(context.callback(Message::Wubi)),
        ),
        field(
            "繁體輸出",
            "打字時將候選詞轉換為繁體中文。關著時，在系統「中文(台灣)」語言下選用的「青簡」仍然輸出繁體，\
             「中文(簡體)」下的「青簡」輸出簡體。",
            ToggleSwitch::new()
                .is_on(g.traditional)
                .on_toggled(context.callback(Message::Traditional)),
        ),
        field(
            "候選後面顯示 emoji",
            "開著時有對應 emoji 的詞後面跟一兩個（kaixin → 開心 😄）；關掉候選裡只有字詞。",
            ToggleSwitch::new()
                .is_on(g.emoji)
                .on_toggled(context.callback(Message::Emoji)),
        ),
        field(
            "中文模式標點轉全形",
            "沒在打拼音時敲 , . ? ! 等出「，。？！」，數字後面的點保持半形；懸浮狀態條的「，。」格也能切，切的是當前模式那份。",
            ToggleSwitch::new()
                .is_on(g.full_width_punctuation)
                .on_toggled(context.callback(Message::FullWidthPunctuation)),
        ),
        field(
            "英文模式標點轉全形",
            "中英各記一份，預設英文半形。",
            ToggleSwitch::new()
                .is_on(g.english_full_width_punctuation)
                .on_toggled(context.callback(Message::EnglishFullWidthPunctuation)),
        ),
        field(
            "組句中標點先上屏候選",
            "打拼音時敲 , . ? ! 等先把高亮候選上屏、再補該標點（nihao, 一氣打完「你好，」）；關掉後標點進英文直輸段（老行為）。",
            ToggleSwitch::new()
                .is_on(g.punct_commits)
                .on_toggled(context.callback(Message::PunctCommits)),
        ),
        field(
            "英文模式（Caps Lock）也給候選",
            "Tab 或方向鍵選詞；空格、回車、標點仍原樣上屏敲的字母，不選詞時與直接打字一樣。",
            ToggleSwitch::new()
                .is_on(g.english_candidates)
                .on_toggled(context.callback(Message::EnglishCandidates)),
        ),
        field(
            "只在瀏覽器、聊天和文件類應用裡給",
            "Chrome、Edge、微信、QQ、Word、記事本等；別的應用（終端、程式碼編輯器、Photoshop 這類靠單鍵快捷鍵的軟體）裡字母原樣交給應用。名單可在配置檔案裡改。",
            ToggleSwitch::new()
                .is_on(english_only_in_apps)
                .is_enabled(g.english_candidates)
                .on_toggled(context.callback(Message::EnglishOnlyInApps)),
        ),
        field(
            "輸入拼音時也給英文詞和補全",
            "整段是英文詞（hello）或像英文詞的開頭（compa → company）時給英文候選；關掉就只出中文，英文模式不受影響。",
            ToggleSwitch::new()
                .is_on(g.english_in_chinese)
                .on_toggled(context.callback(Message::EnglishInChinese)),
        ),
        field(
            "輸入拼音時中文候選排在英文詞前面",
            "開著時整段輸入是英文詞時（hello、key）英文詞排第二，空格上屏的仍是中文；關著（預設）拼音不成立的輸入英文詞排第一。",
            ToggleSwitch::new()
                .is_on(g.chinese_first)
                .is_enabled(g.english_in_chinese)
                .on_toggled(context.callback(Message::ChineseFirst)),
        ),
        feedback(&settings.notice),
        field(
            "中文模式下的 Shift + 字母",
            "「交給應用」是臨時打英文（與以前一致）：拼音先上屏，這個鍵歸應用；\
             「進組句」把它收進拼音緩衝區，匹配時按小寫算，所以 Cpan 與 cpan 一樣能出「C盤」。",
            shift_letter_combo(g.shift_letter, context.callback(Message::ShiftLetter)),
        ),
        field(
            "中英切換鍵",
            "勾上的鍵都能在中英之間切換，可以多選，改完立刻生效；中英模式所有應用共用一份。打字時容易誤觸 Shift 的話改勾「單擊 Ctrl」；一個都不勾時只剩工作列 / 懸浮狀態條上的「中」「英」按鈕。\
             系統自帶的 Ctrl + Space 也能切中英，與微軟拼音一致，不用勾（裝了別的輸入法時 Windows 可能改用它切換輸入法）。",
            switch_key_boxes(settings, context),
        ),
        field(
            "啟用內建英文模式",
            "關掉後青簡固定中文模式：切換鍵與工作列、懸浮狀態條上的「中」「英」按鈕都不再切到英文，需要英文時用系統快捷鍵（Win + Space）切到別的輸入法。",
            ToggleSwitch::new()
                .is_on(g.english_mode)
                .on_toggled(context.callback(Message::EnglishMode)),
        ),
    ];
    page("通用", StackPanel::new().spacing(16.0).children(rows))
}

/// 中英切換鍵：每個鍵一個勾選框，橫排。
fn switch_key_boxes(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let keys = settings.config.shortcut.switch_mode;
    let boxes = SwitchKey::ALL.map(|key| {
        CheckBox::new()
            .is_checked(keys.contains(key))
            .on_is_checked_changed(context.callback(move |on| Message::SwitchKey(key, on)))
            .content(key.label())
    });
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children(boxes)
}
