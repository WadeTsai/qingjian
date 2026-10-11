use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

/// 給程序裝一份只有「編輯」的主選單。
///
/// 輸入法是 `LSBackgroundOnly`，從不顯示選單欄，也就沒有主選單；而 AppKit 文本框裡的 ⌘X / ⌘C / ⌘V / ⌘A / ⌘Z
/// 是主選單裡對應條目的快捷鍵，選單不存在這些鍵就沒人響應——偏好設定裡的金鑰框粘不進去就是這個原因。
/// 選單欄本身永遠不會畫出來（啟用策略最多到 Accessory），只是讓快捷鍵有地方落。重複呼叫只裝一次。
pub fn install(mtm: MainThreadMarker) {
    let app = NSApplication::sharedApplication(mtm);
    if app.mainMenu().is_some() {
        return;
    }
    let edit = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("编辑"));
    for (title, action, key) in [
        ("撤销", sel!(undo:), "z"),
        ("重做", sel!(redo:), "Z"),
        ("剪切", sel!(cut:), "x"),
        ("拷贝", sel!(copy:), "c"),
        ("粘贴", sel!(paste:), "v"),
        ("全选", sel!(selectAll:), "a"),
    ] {
        // SAFETY: 這些都是 NSResponder 的標準編輯選擇器，target 為空時沿響應鏈送到當前編輯的文本框
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str(title),
                Some(action),
                &NSString::from_str(key),
            )
        };
        edit.addItem(&item);
    }
    let main: Retained<NSMenu> = NSMenu::new(mtm);
    let holder = NSMenuItem::new(mtm);
    holder.setSubmenu(Some(&edit));
    main.addItem(&holder);
    app.setMainMenu(Some(&main));
}
