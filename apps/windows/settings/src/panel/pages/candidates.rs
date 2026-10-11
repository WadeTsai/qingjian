//! 「候選視窗」頁：外觀、主題、排布、渲染引擎、字型與字號、過渡動畫、拼音顯示位置、懸浮狀態條。

use qingjian_platform::{Appearance, CandidateRenderer, FontSize, LayoutMode, PreeditMode};
use qingjian_render::ThemeLibrary;
use windows_reactor::*;

use crate::panel::controls::{field, page};
use crate::panel::{Message, Settings};

/// 列舉下拉：按 `label()` 列項，選中 `current`（找不到取 0）。
fn mode_combo<T: PartialEq + Copy>(
    all: &'static [T],
    current: T,
    label: fn(T) -> &'static str,
    callback: Callback<Option<usize>>,
) -> ComboBox {
    ComboBox::new()
        .items_source(all.iter().map(|mode| label(*mode)))
        .selected_index(all.iter().position(|mode| *mode == current).unwrap_or(0))
        .on_selection_changed(callback)
}

/// 字號框：不限範圍；沒設過顯示主題的字號（使用者知道從哪個數開始調）。
fn size_box(size: FontSize, theme: Option<f32>, callback: Callback<Option<f64>>) -> NumberBox {
    NumberBox::new()
        .value(size.get().or(theme).map_or(f64::NAN, f64::from))
        .on_value_changed(callback)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let font_text = settings
        .font_query
        .clone()
        .unwrap_or_else(|| g.font.clone());
    let query = font_text.to_lowercase();
    let suggestions: Vec<String> = settings
        .families
        .iter()
        .filter(|family| family.to_lowercase().contains(&query))
        .cloned()
        .collect();
    // 內建主題加使用者主題目錄裡的，每次畫這一頁都重列（新放進去的主題回到這頁就能看到）
    let themes = ThemeLibrary::load(qingjian_platform::dirs::themes_dir().as_deref());
    let theme = themes.resolve(g.theme_id(), false);
    let rows = [
        field(
            "外觀",
            "",
            mode_combo(
                &Appearance::ALL,
                g.appearance(),
                Appearance::label,
                context.callback(Message::Appearance),
            ),
        ),
        field(
            "主題",
            "只對青簡渲染器生效；櫻花只有淺色，其餘主題按上面的外觀切換淺色與深色。",
            ComboBox::new()
                .items_source(themes.themes().iter().map(|theme| theme.name().to_owned()))
                .selected_index(
                    themes
                        .themes()
                        .iter()
                        .position(|theme| theme.id() == g.theme_id())
                        .unwrap_or(0),
                )
                .on_selection_changed(context.callback(Message::Theme)),
        ),
        field(
            "排布",
            "橫排時只給高亮的候選顯示譯詞。",
            mode_combo(
                &LayoutMode::ALL,
                g.layout,
                LayoutMode::label,
                context.callback(Message::Layout),
            ),
        ),
        field(
            "渲染引擎",
            "青簡渲染器讓候選視窗在各平臺一致。",
            mode_combo(
                &CandidateRenderer::ALL,
                g.renderer,
                CandidateRenderer::label,
                context.callback(Message::Renderer),
            ),
        ),
        field(
            "字型",
            "只對青簡渲染器生效；留空用系統字型，沒裝的字型自動回到系統字型。",
            AutoSuggestBox::new()
                .width(260.0)
                .text(font_text)
                .placeholder_text("系統字型")
                .items_source(suggestions)
                .on_text_changed(context.callback(Message::FontQuery))
                .on_suggestion_chosen(context.callback(Message::Font)),
        ),
        field(
            "候選字號",
            "單位是點，清空或填 0 回到主題的字號；行高跟著縮放。只對青簡渲染器生效。",
            size_box(
                g.candidate_font_size,
                theme.theme_size("candidate"),
                context.callback(Message::CandidateFontSize),
            ),
        ),
        field(
            "譯文字號",
            "單位是點，清空或填 0 回到主題的字號。只對青簡渲染器生效。",
            size_box(
                g.annotation_font_size,
                theme.theme_size("annotation"),
                context.callback(Message::AnnotationFontSize),
            ),
        ),
        field(
            "",
            "",
            Button::new()
                .on_click(context.message(Message::ResetFontSizes))
                .content("恢復主題字號"),
        ),
        field(
            "過渡動畫",
            "高亮換候選時滑過去、主題裡的迴圈動畫。關掉後直接跳到位；系統關了「動畫效果」時也不播。",
            ToggleSwitch::new()
                .is_on(g.animations)
                .on_toggled(context.callback(Message::Animations)),
        ),
        field(
            "拼音顯示",
            "「只在候選視窗」時正在敲的拼音不顯示在應用裡，終端或行內拼音不正常的應用可以選它。",
            mode_combo(
                &PreeditMode::ALL,
                g.preedit,
                PreeditMode::label,
                context.callback(Message::Preedit),
            ),
        ),
        field(
            "懸浮狀態條",
            "桌面上常駐、可拖動的小條：點「中 / 英」切換模式（開著雙拼時還顯示方案名），點「，。」切全形 / 半形標點，點齒輪開啟設定。只在當前輸入法是青簡時顯示，拖到哪下次還在哪。",
            ToggleSwitch::new()
                .is_on(settings.config.status_bar.enabled)
                .on_toggled(context.callback(Message::StatusBar)),
        ),
    ];
    page("候選視窗", StackPanel::new().spacing(16.0).children(rows))
}
