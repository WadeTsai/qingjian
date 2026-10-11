//! 聯想條目前的小云朵：SF Symbols 的 `cloud`，按主題著色；系統太舊拿不到符號時退回文字 `☁︎`。

use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSCompositingOperation, NSImage, NSImageSymbolConfiguration};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

pub struct CloudIcon {
    /// 已著色的符號圖片；`None` 表示用文字兜底。
    image: Option<Retained<NSImage>>,

    /// 繪製尺寸（正方形邊長）。
    size: f64,
}

impl CloudIcon {
    pub fn new(color: &NSColor, size: f64) -> Self {
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str("cloud"),
            Some(&NSString::from_str("云联想")),
        )
        .and_then(|image| {
            let config = NSImageSymbolConfiguration::configurationWithHierarchicalColor(color);
            image.imageWithSymbolConfiguration(&config)
        });
        Self { image, size }
    }

    pub fn is_symbol(&self) -> bool {
        self.image.is_some()
    }

    /// 圖標占的寬度（不含後面的間距）。
    pub fn width(&self) -> f64 {
        self.size
    }

    /// 在翻轉座標系裡畫到 `(x, y)`，`y` 是圖示頂邊。
    pub fn draw(&self, x: f64, y: f64) {
        let Some(image) = &self.image else {
            return;
        };
        let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(self.size, self.size));
        // SAFETY: hints 傳 None，其餘引數都是普通值；在 drawRect: 內呼叫，有當前圖形上下文。
        unsafe {
            image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                rect,
                NSRect::ZERO,
                NSCompositingOperation::SourceOver,
                1.0,
                true,
                None,
            );
        }
    }
}
