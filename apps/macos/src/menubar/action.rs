use objc2_foundation::NSInteger;
use qingjian_core::FuzzyRules;

/// 模糊音條目的 tag 起點，後面加規則在 [`FuzzyRules::NAMES`] 裡的下標。
const FUZZY_TAG_BASE: NSInteger = 100;

/// 選單能觸發的動作。編碼進 NSMenuItem 的 tag，派發時再解出來。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// 開關雲聯想（寫 `[predict] enabled`）。
    ToggleCloud,

    /// 開關一條模糊音規則，值是 [`FuzzyRules::NAMES`] 的下標。
    ToggleFuzzy(usize),

    /// 開啟偏好設定視窗。
    OpenPreferences,

    /// 在訪達裡開啟日誌目錄。
    OpenLogs,

    /// 開啟下載頁（選單裡「有新版本」那一行）。
    OpenDownload,
}

impl MenuAction {
    pub fn tag(self) -> NSInteger {
        match self {
            Self::ToggleCloud => 1,
            Self::OpenPreferences => 2,
            Self::OpenLogs => 3,
            Self::OpenDownload => 4,
            Self::ToggleFuzzy(index) => FUZZY_TAG_BASE + index as NSInteger,
        }
    }

    pub fn from_tag(tag: NSInteger) -> Option<Self> {
        Some(match tag {
            1 => Self::ToggleCloud,
            2 => Self::OpenPreferences,
            3 => Self::OpenLogs,
            4 => Self::OpenDownload,
            _ => {
                let index = usize::try_from(tag.checked_sub(FUZZY_TAG_BASE)?).ok()?;
                (index < FuzzyRules::NAMES.len()).then_some(Self::ToggleFuzzy(index))?
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_round_trip() {
        let all = [
            MenuAction::ToggleCloud,
            MenuAction::OpenPreferences,
            MenuAction::OpenLogs,
            MenuAction::OpenDownload,
            MenuAction::ToggleFuzzy(0),
            MenuAction::ToggleFuzzy(FuzzyRules::NAMES.len() - 1),
        ];
        for action in all {
            assert_eq!(MenuAction::from_tag(action.tag()), Some(action));
        }
        assert_eq!(MenuAction::from_tag(0), None);
        assert_eq!(
            MenuAction::from_tag(FUZZY_TAG_BASE + FuzzyRules::NAMES.len() as NSInteger),
            None
        );
        assert_eq!(MenuAction::from_tag(-1), None);
    }
}
