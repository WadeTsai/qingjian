//! 彈出按鈕子類：長得和其他設定項一樣，但點它不彈選單，彈的是帶搜尋框的字型列表。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSPopUpButton};
use objc2_foundation::NSRect;

use super::source::FontListSource;

/// 檢視狀態。
pub(super) struct ButtonState {
    /// 列表與下拉框的主人。
    pub source: Retained<FontListSource>,
}

define_class!(
    // SAFETY: NSPopUpButton 允許子類化；沒有實現 Drop。
    #[unsafe(super(NSPopUpButton))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ButtonState]
    pub(super) struct FontPopUpButton;

    impl FontPopUpButton {
        /// 滑鼠點下去：不走選單，開我們的下拉框。
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            self.ivars().source.open();
        }

        /// 鍵盤（空格 / 回車）觸發也一樣。
        #[unsafe(method(performClick:))]
        fn perform_click(&self, _sender: Option<&AnyObject>) {
            self.ivars().source.open();
        }
    }
);

impl FontPopUpButton {
    pub(super) fn new(mtm: MainThreadMarker, source: Retained<FontListSource>) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(ButtonState { source });
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO, pullsDown: false] }
    }
}
