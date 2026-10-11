use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEvent, NSEventType, NSText,
    NSTextField, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSRect};

define_class!(
    // SAFETY: NSWindow 允許子類化；沒有實現 Drop。
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    /// 設定視窗的 NSWindow：關窗時把程序的啟用策略切回 Prohibited，讓輸入法回到純後臺。
    /// 文本框只在「結束編輯」時發值，點別的控制元件、切走、關窗都不算結束，改了的值就懸著；這幾處主動結束編輯，與按回車一樣生效。
    pub struct PreferencesPanel;

    impl PreferencesPanel {
        #[unsafe(method(sendEvent:))]
        fn send_event(&self, event: &NSEvent) {
            if event.r#type() == NSEventType::LeftMouseDown && !self.click_in_editing_field(event) {
                self.end_editing();
            }
            let _: () = unsafe { msg_send![super(self), sendEvent: event] };
        }

        #[unsafe(method(resignKeyWindow))]
        fn resign_key_window(&self) {
            self.end_editing();
            let _: () = unsafe { msg_send![super(self), resignKeyWindow] };
        }

        #[unsafe(method(close))]
        fn close(&self) {
            self.end_editing();
            let mtm = MainThreadMarker::from(self);
            NSApplication::sharedApplication(mtm)
                .setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
            let _: () = unsafe { msg_send![super(self), close] };
        }
    }

    unsafe impl NSObjectProtocol for PreferencesPanel {}
);

impl PreferencesPanel {
    pub fn new(mtm: MainThreadMarker, content: NSRect) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        let this: Retained<Self> = unsafe {
            msg_send![
                super(this),
                initWithContentRect: content,
                styleMask: NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        // 程式建的 NSWindow 預設關窗即釋放，我們還握著 Retained，必須關掉
        unsafe { this.setReleasedWhenClosed(false) };
        this
    }

    /// 正在編輯的文本框交出焦點，`sendsActionOnEndEditing` 讓它把值發出去。
    fn end_editing(&self) {
        let editing = self.field_editor().is_some_and(|editor| {
            self.firstResponder().is_some_and(|responder| {
                Retained::as_ptr(&responder).cast::<AnyObject>()
                    == Retained::as_ptr(&editor).cast::<AnyObject>()
            })
        });
        if editing {
            self.makeFirstResponder(None);
        }
    }

    /// 視窗共用的欄位編輯器（文本框編輯時真正收鍵的那個檢視）；還沒建過為 `None`。
    fn field_editor(&self) -> Option<Retained<NSText>> {
        // SAFETY: create 為 false 不新建，物件引數為空取視窗共用的那個
        unsafe { self.fieldEditor_forObject(false, None) }
    }

    /// 點的是正在編輯的那個文本框（繼續編輯，不打斷）。
    fn click_in_editing_field(&self, event: &NSEvent) -> bool {
        let Some(content) = self.contentView() else {
            return false;
        };
        let Some(hit) = content.hitTest(event.locationInWindow()) else {
            return false;
        };
        let Some(editor) = self.field_editor() else {
            return false;
        };
        if hit.isDescendantOf(&editor) {
            return true;
        }
        hit.downcast::<NSTextField>()
            .ok()
            .and_then(|field| field.currentEditor())
            .is_some_and(|current| {
                Retained::as_ptr(&current).cast::<AnyObject>()
                    == Retained::as_ptr(&editor).cast::<AnyObject>()
            })
    }

    /// 切到 Accessory（有視窗、無 Dock 圖示）並把視窗帶到最前，文本框才拿得到鍵盤焦點。
    pub fn present(&self) {
        let mtm = MainThreadMarker::from(self);
        let app = NSApplication::sharedApplication(mtm);
        super::edit_menu::install(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        self.makeKeyAndOrderFront(None);
    }
}
