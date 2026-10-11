//! 各頁共用的表單零件（標籤行、說明小字、整項、頁外殼）與開啟檔案 / 目錄、打包日誌的小工具。

use std::path::{Path, PathBuf};

use windows_reactor::*;

use super::notice::Notice;
use super::{LABEL_WIDTH, Message, Settings};
use crate::log;

/// 隨包資源（相對隨包根，如 `data/generated/dicts`），定位邏輯與 Server 共用。
pub(super) fn repo_resource(rel: &str) -> Option<PathBuf> {
    qingjian_platform::resources::bundled_resource(rel)
}

pub(super) fn open_in_editor(path: &Path) {
    if let Err(error) = std::process::Command::new("notepad").arg(path).spawn() {
        log::warn(format!("開啟 {} 失敗: {error}", path.display()));
    }
}

/// 資源管理器開啟目錄或網址。
pub(super) fn open_with_explorer(target: &str) {
    if let Err(error) = std::process::Command::new("explorer").arg(target).spawn() {
        log::warn(format!("開啟 {target} 失敗: {error}"));
    }
}

/// 三個程序共用的日誌目錄，沒有就建出來（Server 沒跑過時它還不存在）。
pub(super) fn log_dir() -> Option<PathBuf> {
    let dir = qingjian_platform::dirs::log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 把整個日誌目錄加 `config.toml` 打成 `qingjian-logs-<日期>.zip` 放到桌面，再在資源管理器裡選中它——
/// 使用者反饋問題時一個附件搞定。壓縮交給 PowerShell 的 Compress-Archive，不為此拉一個壓縮庫；
/// 桌面路徑也讓 PowerShell 取（OneDrive 會把桌面挪到別處）。指令碼先寫成臨時 .ps1 再跑，免得命令列引號轉義。
pub(super) fn export_logs() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let Some(logs) = log_dir() else {
        return;
    };
    log::warn("使用者匯出日誌");
    let mut sources = vec![format!("'{}\\*'", logs.display())];
    if let Some(config) = qingjian_platform::dirs::config_path().filter(|path| path.is_file()) {
        sources.push(format!("'{}'", config.display()));
    }
    let zip_name = format!(
        "qingjian-logs-{}.zip",
        jiff::Zoned::now().strftime("%Y-%m-%d")
    );
    let script = format!(
        "$zip = Join-Path ([Environment]::GetFolderPath('Desktop')) '{zip_name}'\n\
         Compress-Archive -Path {} -DestinationPath $zip -Force\n\
         explorer.exe \"/select,`\"$zip`\"\"\n",
        sources.join(",")
    );
    let script_path = std::env::temp_dir().join("qingjian-export-logs.ps1");
    if let Err(error) = std::fs::write(&script_path, script) {
        log::warn(format!("寫匯出指令碼失敗: {error}"));
        return;
    }
    let spawned = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
    if let Err(error) = spawned {
        log::warn(format!("匯出日誌失敗: {error}"));
    }
}

/// 一行設定：固定寬標籤 + 控制元件。
pub(super) fn labeled(label: &str, control: impl Into<View>) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children([
            TextBlock::new().text(label).width(LABEL_WIDTH).into(),
            control.into(),
        ])
}

/// 灰色小字說明，可換行。
pub(super) fn note(text: &str) -> View {
    TextBlock::new()
        .text(text)
        .text_wrapping(TextWrapping::Wrap)
        .font_size(12.0)
        .opacity(0.6)
        .into()
}

/// 一整項：「標籤 + 控制元件」一行，下接說明（`hint` 為空則不加）。
pub(super) fn field(label: &str, hint: &str, control: impl Into<View>) -> View {
    let row = labeled(label, control);
    if hint.is_empty() {
        row
    } else {
        StackPanel::new().spacing(4.0).children([row, note(hint)])
    }
}

/// 在 `(介面名, 配置寫法)` 列表裡找 `value` 的下標，找不到取 0。
pub(super) fn index_of(options: &[(&str, &str)], value: &str) -> usize {
    options.iter().position(|(_, v)| *v == value).unwrap_or(0)
}

/// 一行「名稱 · N 條 · 隨包 / 許可證」，壞檔案標出來：詞庫頁與輔碼頁共用。
pub(super) fn entry_title(
    name: &str,
    entries: usize,
    license: &str,
    builtin: bool,
    broken: bool,
) -> String {
    if broken {
        return format!("{name}（檔案損壞）");
    }
    let mut text = format!("{name} · {entries} 條");
    if builtin {
        text.push_str(" · 隨包");
    } else if !license.is_empty() {
        text.push_str(&format!(" · {license}"));
    }
    text
}

/// 一行「核取方塊 + 可選的移除按鈕」：詞庫頁與輔碼頁共用，壞檔案禁掉開關。
pub(super) fn check_row(
    stem: &str,
    label: String,
    enabled: bool,
    broken: bool,
    toggle: impl Fn(bool) -> Message + 'static,
    remove: Option<Message>,
    context: &mut ViewContext<Settings>,
) -> KeyedView {
    let check = CheckBox::new()
        .is_checked(enabled)
        .is_enabled(!broken)
        .on_is_checked_changed(context.callback(toggle))
        .content(label);
    let row = match remove {
        Some(message) => StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                check,
                Button::new()
                    .on_click(context.message(message))
                    .content("移除"),
            )),
        None => check,
    };
    KeyedView::new(stem.to_owned(), row)
}

/// 頁面底部的提示：成功統計一行灰字、失敗一行紅字，都沒有就不畫。
pub(super) fn feedback(notice: &Notice) -> View {
    let mut lines: Vec<KeyedView> = Vec::new();
    if let Some(text) = &notice.note {
        lines.push(KeyedView::new("notice-note", note(text)));
    }
    if let Some(text) = &notice.error {
        lines.push(KeyedView::new(
            "notice-error",
            TextBlock::new()
                .text(text)
                .text_wrapping(TextWrapping::Wrap)
                .font_size(12.0)
                .foreground(ThemeBrush::SystemCritical),
        ));
    }
    StackPanel::new().spacing(4.0).keyed_children(lines)
}

/// 一頁外殼：可滾動 + 大標題 + 內容。
pub(super) fn page(title: &str, body: impl Into<View>) -> View {
    ScrollViewer::new().content(
        StackPanel::new().spacing(16.0).margin(24.0).children([
            TextBlock::new()
                .text(title)
                .font_size(24.0)
                .font_weight(FontWeight::SEMI_BOLD)
                .into(),
            body.into(),
        ]),
    )
}
