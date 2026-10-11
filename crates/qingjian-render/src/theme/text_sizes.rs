//! 設定裡的字號：蓋過主題 `candidate`、`annotation` 兩個文字樣式的字號，行高按同一比例縮放。
//! 序號（`index`）、拼音（`preedit`）與表格行高跟著候選字縮放，同一行才對得齊；主題自定義的其它樣式不動。

use super::font::FontSpec;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextSizes {
    /// 候選字字號（點）；`None` 用主題的。
    pub candidate: Option<f32>,

    /// 譯文字號（點）；`None` 用主題的。
    pub annotation: Option<f32>,
}

impl TextSizes {
    /// 配置裡的數：不是正數的當沒填。
    pub fn new(candidate: f32, annotation: f32) -> Self {
        let positive = |size: f32| (size.is_finite() && size > 0.0).then_some(size);
        Self {
            candidate: positive(candidate),
            annotation: positive(annotation),
        }
    }

    /// `name` 樣式相對主題的縮放比；`theme_size` 是主題裡對應基準樣式（候選或譯文）的字號。
    pub(super) fn ratio(&self, name: &str, theme_size: impl Fn(&str) -> Option<f32>) -> f32 {
        let (wanted, base) = match name {
            "candidate" | "index" | "preedit" => (self.candidate, "candidate"),
            "annotation" => (self.annotation, "annotation"),
            _ => return 1.0,
        };
        match (wanted, theme_size(base)) {
            (Some(wanted), Some(base)) if base > 0.0 => wanted / base,
            _ => 1.0,
        }
    }
}

/// 字號與行高一起乘。
pub(super) fn scaled(spec: FontSpec, ratio: f32) -> FontSpec {
    FontSpec {
        size: spec.size * ratio,
        line_height: spec.line_height * ratio,
        ..spec
    }
}
