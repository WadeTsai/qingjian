//! 候選視窗主題：字型、顏色、間距。所有可視引數集中在這裡，方便以後從配置檔案讀。
//!
//! 視覺層級（產品決定）：候選詞最深，譯文稍淺，詞性最淺，序號弱化。

use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSFont};

pub struct Theme {
    /// 候選詞字型。
    pub text_font: Retained<NSFont>,

    /// 譯文與詞性字型。
    pub annotation_font: Retained<NSFont>,

    /// 序號字型。
    pub index_font: Retained<NSFont>,

    /// 候選詞顏色。
    pub text_color: Retained<NSColor>,

    /// 譯文顏色。
    pub gloss_color: Retained<NSColor>,

    /// 詞性顏色，比譯文更淺。
    pub pos_color: Retained<NSColor>,

    /// 生詞譯文的顏色：比普通譯文醒目，提醒「這個詞你還沒見過幾次」，看熟了就回到譯文色。
    pub fresh_color: Retained<NSColor>,

    /// 序號顏色。
    pub index_color: Retained<NSColor>,

    /// 雲聯想的雲朵與文字顏色：比譯文醒目一點，但仍不搶候選詞。
    pub cloud_color: Retained<NSColor>,

    /// 視窗背景。
    pub background: Retained<NSColor>,

    /// 當前候選的高亮底色。
    pub highlight: Retained<NSColor>,

    /// 視窗內邊距。
    pub padding: f64,

    /// 行內上下留白。
    pub row_padding: f64,

    /// 序號與候選詞、候選詞與譯文之間的間距。
    pub column_gap: f64,

    /// 視窗與高亮條的圓角。
    pub corner_radius: f64,

    /// 最多顯示幾行。
    pub max_rows: usize,
}

impl Theme {
    /// 系統預設外觀。只能在主執行緒呼叫（NSFont / NSColor 不跨執行緒）。
    pub fn system_default() -> Self {
        Self {
            text_font: NSFont::systemFontOfSize(16.0),
            annotation_font: NSFont::systemFontOfSize(12.0),
            index_font: NSFont::systemFontOfSize(11.0),
            text_color: NSColor::labelColor(),
            gloss_color: NSColor::secondaryLabelColor(),
            pos_color: NSColor::tertiaryLabelColor(),
            fresh_color: NSColor::systemOrangeColor(),
            index_color: NSColor::tertiaryLabelColor(),
            cloud_color: NSColor::systemTealColor(),
            background: NSColor::windowBackgroundColor(),
            highlight: NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.48, 1.0, 0.16),
            padding: 8.0,
            row_padding: 4.0,
            column_gap: 8.0,
            corner_radius: 8.0,
            max_rows: 9,
        }
    }
}
