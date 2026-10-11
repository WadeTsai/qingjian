//! 「快捷鍵」頁：翻頁鍵、模式鍵，譯詞 / 刪候選 / 翻譯選中文字的修飾鍵。
//! 翻譯選中文字只改修飾鍵，字母鍵固定用配置裡當前的；要換字母直接改 `config.toml`。

use qingjian_platform::Modifiers;
use windows_reactor::*;

use crate::panel::controls::{field, index_of, page};
use crate::panel::{Message, Settings};

/// 翻頁鍵對：介面名 + 配置寫法。
pub(crate) const PAGE_KEYS: [(&str, &str); 3] = [
    ("方括號 [ ]", "[]"),
    ("逗號句號 , .", ",."),
    ("減號等號 - =", "-="),
];

/// 可當模式鍵的字母（與 Core `ModeKeys::CANDIDATES` 一致）。
pub(crate) const MODE_KEYS: [&str; 3] = ["v", "u", "i"];

/// 修飾鍵預設：介面名 + 配置寫法。
pub(crate) const MODIFIERS: [(&str, &str); 6] = [
    ("Ctrl", "ctrl"),
    ("Alt", "alt"),
    ("Shift", "shift"),
    ("Ctrl + Shift", "shift+ctrl"),
    ("Ctrl + Alt", "ctrl+alt"),
    ("Alt + Shift", "shift+alt"),
];

fn mode_combo(current: char, callback: Callback<Option<usize>>) -> ComboBox {
    let selected = MODE_KEYS
        .iter()
        .position(|key| key.starts_with(current))
        .unwrap_or(0);
    ComboBox::new()
        .items_source(MODE_KEYS)
        .selected_index(selected)
        .on_selection_changed(callback)
}

/// 按解析後相等找當前項，不依賴字串寫法。
fn modifier_combo(current: Modifiers, callback: Callback<Option<usize>>) -> ComboBox {
    let selected = MODIFIERS
        .iter()
        .position(|(_, value)| value.parse::<Modifiers>().ok() == Some(current))
        .unwrap_or(0);
    ComboBox::new()
        .items_source(MODIFIERS.iter().map(|(label, _)| *label))
        .selected_index(selected)
        .on_selection_changed(callback)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let s = &settings.config.shortcut;
    let rows = [
        field(
            "翻頁鍵",
            "選「, .」或「- =」時組句中敲對應符號是翻頁，不再是上屏加標點。",
            ComboBox::new()
                .items_source(PAGE_KEYS.iter().map(|(label, _)| *label))
                .selected_index(index_of(&PAGE_KEYS, &settings.config.general.page_keys))
                .on_selection_changed(context.callback(Message::PageKeys)),
        ),
        field(
            "表示式模式鍵",
            "",
            mode_combo(s.mode.expression, context.callback(Message::ModeExpression)),
        ),
        field(
            "問字模式鍵",
            "這兩個字母開頭進模式：v1+2 出 3，usangemu 問「三個木」（需要雲服務）。兩個鍵不能相同。",
            mode_combo(s.mode.question, context.callback(Message::ModeQuestion)),
        ),
        field(
            "沒在輸入拼音時敲 ? 也進入問字",
            "開著時 ? 先進問字（中英文模式都行），後面跟字母才是問題，跟其他鍵時還原成問號；關著問號就是問號。",
            ToggleSwitch::new()
                .is_on(s.mode.question_mark)
                .on_toggled(context.callback(Message::QuestionMark)),
        ),
        field(
            "譯詞上屏（第一個）",
            "按住修飾鍵再按候選序號，上屏候選右側的譯詞而不是中文。",
            modifier_combo(s.translation, context.callback(Message::Translation)),
        ),
        field(
            "譯詞上屏（第二個）",
            "候選有兩個譯詞時，這組鍵上屏後一個。兩組不能相同。Ctrl + Shift 是 Windows 切換輸入法的熱鍵，同時裝著別的輸入法時別選它。",
            modifier_combo(
                s.translation_second,
                context.callback(Message::TranslationSecond),
            ),
        ),
        field(
            "刪除候選",
            "按住修飾鍵再按候選序號：自己造的詞、雲端選過的詞整刪；詞庫裡的詞清掉學習記錄，回到原排序。",
            modifier_combo(
                s.delete_candidate,
                context.callback(Message::DeleteCandidate),
            ),
        ),
        field(
            "翻譯選中文字",
            "選中一段文字後按這組鍵 + 當前字母（預設 Ctrl+Alt+T），把它譯成學習語言，回車 / 空格替換、Esc 保留原文。需要雲服務。這裡只改修飾鍵，字母固定用當前的。",
            modifier_combo(
                s.translate_selection.modifiers,
                context.callback(Message::TranslateSelection),
            ),
        ),
    ];
    page("快捷鍵", StackPanel::new().spacing(16.0).children(rows))
}
