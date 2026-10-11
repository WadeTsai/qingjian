use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::host;

define_class!(
    // SAFETY: NSObject 沒有子類化要求；沒有實現 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    /// 設定視窗所有控制元件的 target：一個 `changed:` 選擇器，靠 tag 區分控制元件。
    pub struct PreferencesTarget;

    impl PreferencesTarget {
        #[unsafe(method(editPhrase:))]
        fn edit_phrase(&self, _sender: Option<&AnyObject>) {
            host::with(|h| h.change_setting(super::Setting::EditPhrase, super::SettingValue::Bool(false)));
        }

        #[unsafe(method(changed:))]
        fn changed(&self, sender: Option<&AnyObject>) {
            if let Some((setting, value)) = super::setting_from_sender(sender) {
                host::with(|h| h.change_setting(setting, value));
            }
        }
    }

    unsafe impl NSObjectProtocol for PreferencesTarget {}
);

impl PreferencesTarget {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
