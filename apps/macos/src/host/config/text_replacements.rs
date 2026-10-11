//! 系統的文本替換（系統設定「鍵盤 → 文本替換」）：從全域性偏好讀出來，Host 把它並進自定義短語。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSUserDefaults};

/// 全域性偏好裡存文本替換的鍵（`defaults read -g NSUserDictionaryReplacementItems`）：
/// 每條是 `{ on, replace, with }` 字典，iCloud 同步過來的也在這裡。
const REPLACEMENTS_KEY: &str = "NSUserDictionaryReplacementItems";

/// 一條替換：輸入碼與短語。
pub type TextReplacement = (String, String);

/// 讀系統當前啟用的文本替換；沒設過或結構不認識就是空。內容可能含地址、證件號，不記日誌。
pub fn read_system() -> Vec<TextReplacement> {
    let defaults = NSUserDefaults::standardUserDefaults();
    let Some(items) = defaults.arrayForKey(&NSString::from_str(REPLACEMENTS_KEY)) else {
        return Vec::new();
    };
    items.iter().filter_map(|item| parse_item(&item)).collect()
}

/// 一條 `{ on, replace, with }`：`on` 預設算開；`replace` / `with` 不是字串的跳過。
fn parse_item(item: &AnyObject) -> Option<TextReplacement> {
    let dict = item.downcast_ref::<NSDictionary>()?;
    let enabled = dict
        .objectForKey(&NSString::from_str("on"))
        .and_then(|value| value.downcast::<NSNumber>().ok())
        .is_none_or(|number| number.as_bool());
    if !enabled {
        return None;
    }
    let replace = string_for(dict, "replace")?;
    let with = string_for(dict, "with")?;
    Some((replace, with))
}

fn string_for(dict: &NSDictionary, key: &str) -> Option<String> {
    let value: Retained<AnyObject> = dict.objectForKey(&NSString::from_str(key))?;
    let text = value.downcast::<NSString>().ok()?;
    Some(text.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_the_global_list_without_panicking() {
        // 這臺機器上有沒有條目都行；有的話輸入碼與短語都非空（`on = 0` 的已被過濾）
        let items = super::read_system();
        eprintln!("系统文本替换 {} 条", items.len());
        for (code, text) in &items {
            assert!(!code.is_empty() && !text.is_empty());
        }
    }
}
