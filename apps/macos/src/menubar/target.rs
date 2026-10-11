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
    /// 狀態項選單的 target：輸入法程序沒有 key window，nil target 走不到響應鏈，得有個實體收 action。
    /// 輸入源選單那邊的同名選擇器在控制器上，兩邊都匯到 [`host::Host::perform`]。
    pub struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(menuAction:))]
        fn menu_action(&self, sender: Option<&AnyObject>) {
            if let Some(action) = super::action_from_sender(sender) {
                host::with(|h| h.perform(action));
            }
        }
    }

    unsafe impl NSObjectProtocol for MenuTarget {}
);

impl MenuTarget {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
