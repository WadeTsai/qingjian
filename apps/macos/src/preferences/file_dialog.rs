use objc2::MainThreadMarker;
use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
use objc2_foundation::NSString;
use std::path::PathBuf;

/// 彈系統的開啟檔案對話方塊選一個詞庫檔案（TSV / Rime yaml / .qj）。使用者取消返回 `None`。
/// 模態執行，只在偏好設定視窗開啟時呼叫（那時程序已是 Accessory，能出對話方塊）。
pub fn choose_dictionary_file() -> Option<PathBuf> {
    let mtm = MainThreadMarker::new()?;
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(false);
    panel.setAllowsMultipleSelection(false);
    panel.setMessage(Some(&NSString::from_str(
        "选择要导入的词库：青简 TSV、Rime .dict.yaml 或 .qj",
    )));
    panel.setPrompt(Some(&NSString::from_str("导入")));
    if panel.runModal() != NSModalResponseOK {
        return None;
    }
    let urls = panel.URLs();
    let url = urls.firstObject()?;
    let path = url.path()?;
    Some(PathBuf::from(path.to_string()))
}
