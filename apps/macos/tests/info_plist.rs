//! Info.plist 與 Text Input Sources 的契約測試。
//!
//! 缺輸入模式宣告（`ComponentInputModeDict`）時：標準文本檢視（備忘錄等）按 script / 模式過濾輸入源，
//! 會話被立即拆除、表現為「切換不過去」（#31）；系統設定「新增輸入法」列表也不顯示青簡，
//! 安裝器自動啟用失敗後用戶沒有兜底路徑（macOS 26 實測）。打包指令碼把這個檔案原樣拷進 `.app`，
//! 所以在這裡守住宣告結構與身份鍵的對應關係，改壞的人 `cargo test -p qingjian-macos` 立刻能看見。

use std::path::PathBuf;

use plist::{Dictionary, Value};

/// 讀倉庫裡的 `apps/macos/Info.plist`。
fn info_plist() -> Dictionary {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Info.plist");
    Value::from_file(&path)
        .unwrap_or_else(|error| panic!("读不到 {}: {error}", path.display()))
        .into_dictionary()
        .expect("Info.plist 顶层必须是 dict")
}

/// 取字串鍵，缺了或型別不對就地報鍵名。
fn string<'a>(dict: &'a Dictionary, key: &str) -> &'a str {
    dict.get(key)
        .and_then(|value| value.as_string())
        .unwrap_or_else(|| panic!("Info.plist 缺字符串键 {key}"))
}

/// 取字串陣列鍵，缺了或型別不對就地報鍵名。
fn string_list<'a>(dict: &'a Dictionary, key: &str) -> Vec<&'a str> {
    dict.get(key)
        .and_then(|value| value.as_array())
        .unwrap_or_else(|| panic!("Info.plist 缺数组键 {key}"))
        .iter()
        .map(|value| {
            value
                .as_string()
                .unwrap_or_else(|| panic!("{key} 的元素必须是字符串"))
        })
        .collect()
}

/// TIS 登記輸入源用的頂層身份鍵：連線名按 IMK 10.7 起的約定必須是 `<bundle id>_Connection`。
#[test]
fn tis_registration_keys_are_present() {
    let plist = info_plist();
    let id = string(&plist, "TISInputSourceID");
    assert_eq!(
        id,
        string(&plist, "CFBundleIdentifier"),
        "TISInputSourceID 要与 bundle id 一致，系统按它登记输入源"
    );
    assert_eq!(
        string(&plist, "InputMethodConnectionName"),
        format!("{id}_Connection"),
        "连接名必须是 <bundle id>_Connection，与 IMKServer 创建参数对得上"
    );
    assert!(
        !string(&plist, "InputMethodServerControllerClass").is_empty(),
        "controller 类名不能为空"
    );
    assert_eq!(string(&plist, "InputMethodType"), "Keyboard");
    let repertoire = string_list(&plist, "tsInputMethodCharacterRepertoireKey");
    assert!(
        repertoire.contains(&"Hans"),
        "顶层 tsInputMethodCharacterRepertoireKey 要含 Hans：{repertoire:?}"
    );
}

/// 輸入模式宣告：每個模式的 ID、語言、repertoire、script 與圖示都要與頂層宣告對得上，可見模式順序表要列全，
/// 顯示名要在兩份 InfoPlist.strings 裡按模式 ID 給。聲明裡具體幾個模式不限定（將來加繁體模式也過）。
#[test]
fn declares_input_modes_for_standard_text_views() {
    let plist = info_plist();
    let parent = string(&plist, "TISInputSourceID");
    let method_repertoire = string_list(&plist, "tsInputMethodCharacterRepertoireKey");
    let icon = string(&plist, "tsInputMethodIconFileKey");
    let component = plist
        .get("ComponentInputModeDict")
        .and_then(|value| value.as_dictionary())
        .expect(
            "缺 ComponentInputModeDict：没有输入模式声明，标准文本视图（备忘录等）切换不过去，\
             系统设置「添加输入法」列表也不显示青简（#31）",
        );
    let modes = component
        .get("tsInputModeListKey")
        .and_then(|value| value.as_dictionary())
        .expect("ComponentInputModeDict 里缺 tsInputModeListKey");
    assert!(!modes.is_empty(), "tsInputModeListKey 不能是空 dict");
    let visible = string_list(component, "tsVisibleInputModeOrderedArrayKey");
    assert!(
        !visible.is_empty(),
        "tsVisibleInputModeOrderedArrayKey 不能为空：--register 启用的是它的第一项"
    );
    let strings = [localized_strings("zh-Hans"), localized_strings("en")];
    for (mode_id, value) in modes {
        let mode = value
            .as_dictionary()
            .unwrap_or_else(|| panic!("模式 {mode_id} 的值必须是 dict"));
        assert_eq!(
            string(mode, "TISInputSourceID"),
            mode_id,
            "模式字典的键要与它声明的 TISInputSourceID 一致"
        );
        assert!(
            mode_id.starts_with(&format!("{parent}.")),
            "模式 ID {mode_id} 要以 {parent}. 开头"
        );
        assert_eq!(
            string(mode, "TISIntendedLanguage"),
            string(&plist, "TISIntendedLanguage"),
            "模式语言要与顶层一致"
        );
        let repertoire = string_list(mode, "tsInputModeCharacterRepertoireKey");
        assert!(
            !repertoire.is_empty(),
            "模式 {mode_id} 的 repertoire 不能为空"
        );
        for script in &repertoire {
            assert!(
                method_repertoire.contains(script),
                "模式 repertoire {script} 没有在顶层 tsInputMethodCharacterRepertoireKey 里声明"
            );
        }
        assert_eq!(
            string(mode, "tsInputModeScriptKey"),
            "smUnicodeScript",
            "Hans 模式挂在 Unicode script 下，写错 script 标准文本视图认不出"
        );
        for key in [
            "tsInputModeDefaultStateKey",
            "tsInputModeIsVisibleKey",
            "tsInputModePrimaryInScriptKey",
        ] {
            assert_eq!(
                mode.get(key).and_then(|value| value.as_boolean()),
                Some(true),
                "模式 {mode_id} 的 {key} 必须为 true，否则默认不随输入法启用、输入法菜单不显示"
            );
        }
        for key in [
            "tsInputModeMenuIconFileKey",
            "tsInputModeAlternateMenuIconFileKey",
            "tsInputModePaletteIconFileKey",
        ] {
            assert_eq!(
                string(mode, key),
                icon,
                "模式的 {key} 与输入法菜单图标用同一个文件（打包脚本只拷这一个 pdf）"
            );
        }
        assert!(
            visible.contains(&mode_id.as_str()),
            "模式 {mode_id} 没列进 tsVisibleInputModeOrderedArrayKey"
        );
        for (locale, table) in &strings {
            assert!(
                table.contains(&format!("\"{mode_id}\"")),
                "{locale}.lproj/InfoPlist.strings 里缺模式 {mode_id} 的显示名，系统会把 ID 原样当名字"
            );
        }
    }
}

/// 讀 `resources/<locale>.lproj/InfoPlist.strings` 原文。
fn localized_strings(locale: &str) -> (String, String) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join(format!("{locale}.lproj"))
        .join("InfoPlist.strings");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("读不到 {}: {error}", path.display()));
    (locale.to_owned(), text)
}
