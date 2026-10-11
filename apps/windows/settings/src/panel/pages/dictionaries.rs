//! 「詞庫」頁：隨包領域詞庫開關，使用者匯入詞庫的開關 / 移除 / 匯入。
//! 隨包開關寫 `[dictionaries] domains`（列開啟的），使用者詞庫寫 `disabled`（列關掉的）。

use std::path::{Path, PathBuf};

use qingjian_dictionary::Dictionary;
use qingjian_platform::extra_dictionaries;
use windows_reactor::*;

use crate::panel::controls::{check_row, entry_title, note, page, repo_resource};
use crate::panel::{Message, Settings};

/// 使用者詞庫目錄 `%APPDATA%\Qingjian\dicts`。
fn user_dir(settings: &Settings) -> PathBuf {
    settings.data_dir().join("dicts")
}

/// 開啟詞庫讀顯示資訊：(顯示名, 詞條數, 許可證, 是否壞檔案)。
fn read_info(path: &Path, stem: &str) -> (String, usize, String, bool) {
    match Dictionary::from_path(path) {
        Ok(dict) => (
            dict.metadata()
                .map_or_else(|| stem.to_owned(), |m| m.name.clone()),
            dict.len(),
            dict.metadata()
                .map_or_else(String::new, |m| m.license.clone()),
            false,
        ),
        Err(_) => (stem.to_owned(), 0, String::new(), true),
    }
}

fn bundled_list(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let Some(dir) = repo_resource("data/generated/dicts") else {
        return note("沒找到隨包領域詞庫目錄（安裝佈局待定）。");
    };
    let dicts = extra_dictionaries::list(&dir);
    if dicts.is_empty() {
        return note("隨包領域詞庫目錄是空的。");
    }
    let mut rows: Vec<KeyedView> = Vec::with_capacity(dicts.len());
    for (stem, path) in dicts {
        let (name, entries, license, broken) = read_info(&path, &stem);
        let enabled = settings.config.dictionaries.is_domain_enabled(&stem);
        let label = entry_title(&name, entries, &license, true, broken);
        let for_msg = stem.clone();
        rows.push(check_row(
            &stem,
            label,
            enabled,
            broken,
            move |on| Message::ToggleDomain(for_msg.clone(), on),
            None,
            context,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(rows)
}

fn user_list(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let dicts = extra_dictionaries::list(&user_dir(settings));
    if dicts.is_empty() {
        return note(
            "還沒有匯入詞庫。點下面「匯入詞庫」加一本，或把檔案放進 %APPDATA%\\Qingjian\\dicts。",
        );
    }
    let mut rows: Vec<KeyedView> = Vec::with_capacity(dicts.len());
    for (stem, path) in dicts {
        let (name, entries, license, broken) = read_info(&path, &stem);
        let enabled = settings.config.dictionaries.is_enabled(&stem);
        let label = entry_title(&name, entries, &license, false, broken);
        let for_msg = stem.clone();
        let remove = Message::RemoveUserDict(stem.clone());
        rows.push(check_row(
            &stem,
            label,
            enabled,
            broken,
            move |on| Message::ToggleUserDict(for_msg.clone(), on),
            Some(remove),
            context,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(rows)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let body = StackPanel::new().spacing(12.0).children([
        note("隨包的基礎詞庫始終啟用，不在這裡。這裡管隨包領域詞庫的開關與匯入詞庫的開關 / 移除。改完自動生效。"),
        TextBlock::new()
            .text("隨包領域詞庫")
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        bundled_list(settings, context),
        TextBlock::new()
            .text("匯入的詞庫")
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        user_list(settings, context),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                Button::new()
                    .on_click(context.message(Message::ImportDictionary))
                    .content("匯入詞庫…"),
                note("接受青簡 TSV、Rime .dict.yaml 與現成的 .qj；匯入即轉換進上面的目錄，同名覆蓋。"),
            )),
        note(&settings.dictionary_status),
    ]);
    page("詞庫", body)
}

/// 挪進 `dicts\removed`，不真刪（與 macOS 一致）。
pub(crate) fn remove_user_dict(settings: &mut Settings, stem: &str) {
    let dir = user_dir(settings);
    let Some((_, path)) = extra_dictionaries::list(&dir)
        .into_iter()
        .find(|(name, _)| name == stem)
    else {
        return;
    };
    let removed = dir.join("removed");
    if let Err(error) = std::fs::create_dir_all(&removed) {
        settings.dictionary_status = format!("移除失敗：{error}");
        return;
    }
    if let Some(file_name) = path.file_name() {
        settings.dictionary_status = match std::fs::rename(&path, removed.join(file_name)) {
            Ok(()) => format!("已移除「{stem}」，輸入法將自動更新。"),
            Err(error) => format!("移除失敗：{error}"),
        };
    }
}

/// 多選詞庫，逐個轉換並彙總結果；成功項的開關一次寫回。
pub(crate) fn import(settings: &mut Settings) {
    let Some(sources) = rfd::FileDialog::new()
        .add_filter("詞庫檔案", &["tsv", "yaml", "yml", "qj"])
        .add_filter("所有檔案", &["*"])
        .set_title("匯入詞庫")
        .pick_files()
    else {
        return;
    };
    let dir = user_dir(settings);
    let mut disabled = settings.config.dictionaries.disabled.clone();
    let mut results = Vec::new();
    let mut succeeded = 0;
    for source in &sources {
        match qingjian_core::dictionary::import::import(source, &dir) {
            Ok(imported) => {
                if let Some(stem) = imported.path.file_stem().and_then(|s| s.to_str()) {
                    disabled.retain(|name| name != stem);
                }
                succeeded += 1;
                results.push(format!(
                    "已匯入「{}」，共 {} 條。",
                    imported.name, imported.entries
                ));
            }
            Err(error) => {
                let message = format!("{} 匯入失敗：{error}", source.display());
                crate::log::warn(&message);
                results.push(message);
            }
        }
    }
    let mut summary = format!(
        "匯入完成：成功 {} 個，失敗 {} 個。",
        succeeded,
        sources.len() - succeeded
    );
    if disabled != settings.config.dictionaries.disabled
        && let Err(error) = qingjian_platform::Config::set_array(
            &settings.path,
            "dictionaries",
            "disabled",
            &disabled,
        )
    {
        results.push(format!(
            "自動啟用失敗：{error}。此前關閉的詞庫需手動勾選啟用。"
        ));
    } else if succeeded > 0 {
        summary.push_str("輸入法將自動載入。");
    }
    settings.dictionary_status = format!("{summary}\n{}", results.join("\n"));
}
