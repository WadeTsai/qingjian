use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuItem};
use objc2_foundation::NSString;
use qingjian_core::FuzzyRules;
use qingjian_platform::Config;

use super::MenuAction;
use super::target::MenuTarget;

/// 選單本體與需要按狀態重新整理的那幾項。
pub struct InputMenu {
    /// 選單。掛到狀態項和 IMK `menu` 回撥的是同一個物件。
    menu: Retained<NSMenu>,

    /// 「雲聯想」勾選項。
    cloud: Retained<NSMenuItem>,

    /// 模糊音子選單的九條勾選項，順序同 [`FuzzyRules::NAMES`]。
    fuzzy: Vec<Retained<NSMenuItem>>,

    /// 配置檔案解析失敗時顯示的提示行，平時隱藏。
    error: Retained<NSMenuItem>,

    /// 「有新版本 x.y.z…」，點了開啟下載頁；沒有新版時隱藏。
    update: Retained<NSMenuItem>,

    /// 所有條目的 target，要和選單活得一樣久。
    _target: Retained<MenuTarget>,
}

impl InputMenu {
    pub fn new(mtm: MainThreadMarker, version: &str) -> Self {
        let target = MenuTarget::new(mtm);
        let menu = NSMenu::new(mtm);
        // 不讓 AppKit 按響應鏈判斷可用性：它找不到 target 就會把整份選單灰掉
        menu.setAutoenablesItems(false);

        let cloud = action_item(mtm, "云联想", Some(MenuAction::ToggleCloud), &target);
        menu.addItem(&cloud);

        let fuzzy_menu = NSMenu::new(mtm);
        fuzzy_menu.setAutoenablesItems(false);
        let fuzzy: Vec<_> = FuzzyRules::NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let item = action_item(
                    mtm,
                    &fuzzy_label(name),
                    Some(MenuAction::ToggleFuzzy(index)),
                    &target,
                );
                fuzzy_menu.addItem(&item);
                item
            })
            .collect();
        let fuzzy_parent = action_item(mtm, "模糊音", None, &target);
        fuzzy_parent.setSubmenu(Some(&fuzzy_menu));
        menu.addItem(&fuzzy_parent);

        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&action_item(
            mtm,
            "偏好设置…",
            Some(MenuAction::OpenPreferences),
            &target,
        ));
        menu.addItem(&action_item(
            mtm,
            "打开日志目录",
            Some(MenuAction::OpenLogs),
            &target,
        ));
        // 可點的條目只能放在這一組：放到下面兩個純展示條目之間，IMK 會在每次按鍵後停用再新建會話，打不了字
        let update = action_item(mtm, "", Some(MenuAction::OpenDownload), &target);
        update.setHidden(true);
        menu.addItem(&update);
        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let error = action_item(mtm, "", None, &target);
        error.setEnabled(false);
        error.setHidden(true);
        menu.addItem(&error);
        let about = action_item(mtm, &format!("青简 {version}"), None, &target);
        about.setEnabled(false);
        menu.addItem(&about);

        Self {
            menu,
            cloud,
            fuzzy,
            error,
            update,
            _target: target,
        }
    }

    /// 查到新版本就露出「有新版本」那一行，沒有就藏起來。
    pub fn sync_update(&self, available: Option<&str>) {
        match available {
            Some(version) => {
                self.update
                    .setTitle(&NSString::from_str(&format!("有新版本 {version}…")));
                self.update.setHidden(false);
            }
            None => self.update.setHidden(true),
        }
    }

    /// 給狀態項 / IMK 回撥用的選單物件。
    pub fn ns_menu(&self) -> Retained<NSMenu> {
        self.menu.clone()
    }

    /// 按當前配置重新整理勾選狀態。`cloud_active` 是 Engine 裡真接上了 Predictor：
    /// 配置開了但沒接上（多半是沒金鑰）時不打勾，標題說明原因，不能顯示開了實際沒開。
    pub fn sync(&self, config: &Config, cloud_active: bool, error: Option<&str>) {
        let title = match (config.predict.enabled, cloud_active) {
            (true, false) => "云联想（启用失败，见日志）",
            _ => "云联想",
        };
        self.cloud.setTitle(&NSString::from_str(title));
        set_checked(&self.cloud, cloud_active);
        for (item, name) in self.fuzzy.iter().zip(FuzzyRules::NAMES) {
            set_checked(item, config.fuzzy.is_on(name));
        }
        match error {
            Some(message) => {
                self.error
                    .setTitle(&NSString::from_str(&format!("配置文件有错误：{message}")));
                self.error.setHidden(false);
            }
            None => self.error.setHidden(true),
        }
    }
}

/// 建一個選單項。`action` 為 `None` 的是純展示項（子選單父項、關於行）。
fn action_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<MenuAction>,
    target: &MenuTarget,
) -> Retained<NSMenuItem> {
    // SAFETY: 選擇器與 MenuTarget / 控制器上定義的 `menuAction:` 一致，簽名 (id) -> void
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(title),
            action.map(|_| sel!(menuAction:)),
            &NSString::from_str(""),
        )
    };
    if let Some(action) = action {
        unsafe { item.setTarget(Some(target)) };
        item.setTag(action.tag());
    }
    item
}

fn set_checked(item: &NSMenuItem, on: bool) {
    item.setState(if on {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
}

/// `an_ang` → `an = ang`。
fn fuzzy_label(name: &str) -> String {
    name.replace('_', " = ")
}
