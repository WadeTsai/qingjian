//! 「輔碼」頁：觸發鍵錄製、候選上顯示碼開關、碼錶列表（隨包 + 匯入）與匯入碼錶。
//! 觸發鍵與顯示碼寫 `[general]`，碼錶開關寫 `[aux_code] disabled`（列關掉的，與 `[dictionaries] disabled` 同形）。

use std::path::PathBuf;

use qingjian_core::is_valid_aux_code_key;
use qingjian_dictionary::{DictionaryError, aux_code_table_info, import_aux_code_table};
use qingjian_platform::code_tables;
use windows_reactor::*;

use crate::panel::controls::{check_row, entry_title, feedback, field, note, page, repo_resource};
use crate::panel::recorder::Recorder;
use crate::panel::{Message, Settings};

/// 隨包碼錶目錄：隨包根下的 `data/generated/codes/`（與 `dicts/` 並列），與 Server 裝配同款。
const BUNDLED_DIR: &str = "data/generated/codes";

/// 使用者匯入的碼錶目錄 `%APPDATA%\Qingjian\codes`。
fn user_dir(settings: &Settings) -> PathBuf {
    settings.data_dir().join("codes")
}

/// 觸發鍵控制元件：平時是顯示當前鍵的按鈕，點下去換成等按鍵的錄製框。
fn record_control(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    match settings.recorder {
        Recorder::Idle => Button::new()
            .on_click(context.message(Message::AuxRecordStart))
            .content(settings.config.general.aux_code_key().to_string()),
        Recorder::Waiting { attempt } => {
            // 密碼框承接按鍵：不走輸入法，按 A–Z 不彈候選窗，字元直接進來（Shift 與鍵盤佈局由系統處理）。
            // 一輪一代：敲過一次鍵就重建，框裡不留上一輪的字元
            let record_box = settings.record_box.clone();
            context.use_effect("aux-code-record", attempt, move || {
                let _ = record_box.request_focus();
                None
            });
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .keyed_children([
                    KeyedView::new(
                        format!("record-{attempt}"),
                        PasswordBox::new()
                            .element_ref(&settings.record_box)
                            .placeholder_text("請按一個鍵")
                            .password_reveal_mode(PasswordRevealMode::Hidden)
                            .width(160.0)
                            .on_password_changed(context.callback(Message::AuxRecorded)),
                    ),
                    KeyedView::new(
                        "record-cancel",
                        Button::new()
                            .on_click(context.message(Message::AuxRecordCancel))
                            .content("取消"),
                    ),
                ])
        }
    }
}

/// 隨包碼錶：標「隨包」，不給移除（開關還是能關）。
fn bundled_list(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let Some(dir) = repo_resource(BUNDLED_DIR) else {
        return note(
            "沒找到隨包碼錶目錄（隨包根的 data\\generated\\codes\\），隨包筆畫表裝好後這裡會列出來。",
        );
    };
    let tables = code_tables::list(&dir);
    if tables.is_empty() {
        return note("隨包碼錶目錄是空的。");
    }
    let mut rows: Vec<KeyedView> = Vec::with_capacity(tables.len());
    for (stem, path) in tables {
        let info = aux_code_table_info(&path);
        let enabled = settings.config.aux_code.is_enabled(&stem);
        let label = entry_title(&info.name, info.entries, &info.license, true, info.broken);
        let for_msg = stem.clone();
        rows.push(check_row(
            &stem,
            label,
            enabled,
            info.broken,
            move |on| Message::ToggleAuxTable(for_msg.clone(), on),
            None,
            context,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(rows)
}

/// 自己匯入的碼錶：可開關、可移除（挪進 codes\removed，不真刪）。
fn user_list(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let dir = user_dir(settings);
    let tables = code_tables::list(&dir);
    if tables.is_empty() {
        return note(
            "還沒有匯入碼錶。點下面「匯入碼錶」加一張，或把 .qj 放進 %APPDATA%\\Qingjian\\codes。",
        );
    }
    let mut rows: Vec<KeyedView> = Vec::with_capacity(tables.len());
    for (stem, path) in tables {
        let info = aux_code_table_info(&path);
        let enabled = settings.config.aux_code.is_enabled(&stem);
        let label = entry_title(&info.name, info.entries, &info.license, false, info.broken);
        let for_msg = stem.clone();
        let remove = Message::RemoveAuxTable(stem.clone());
        rows.push(check_row(
            &stem,
            label,
            enabled,
            info.broken,
            move |on| Message::ToggleAuxTable(for_msg.clone(), on),
            Some(remove),
            context,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(rows)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let body = StackPanel::new().spacing(12.0).children([
        note(
            "拼音打完之後敲觸發鍵進輔碼態，再敲碼錶裡的字母把候選縮到這些碼上；碼錶沒覆蓋的詞在輔碼態不出現。改完自動生效。",
        ),
        field(
            "啟用輔碼",
            "開著時拼音打完之後敲觸發鍵進輔碼態；關著時觸發鍵完全保持原生行為（英文直輸段 / 雙拼韻母鍵）。沒有任何可用碼錶時也不會進輔碼態。",
            ToggleSwitch::new()
                .is_on(settings.config.aux_code.enabled)
                .on_toggled(context.callback(Message::AuxCodeEnabled)),
        ),
        field(
            "觸發鍵",
            "點一下再按一個鍵；字母、數字、單引號與當前翻頁鍵不能當觸發鍵，微軟、搜狗雙拼裡分號先當 ing 的韻母。",
            record_control(settings, context),
        ),
        field(
            "候選上顯示碼",
            "碼用方括號緊跟在候選詞後面，顯示成「鶴[rbm] crane」。開著時不用進輔碼：純拼音打字候選也帶詞的碼（首條），方便邊打邊記。",
            ToggleSwitch::new()
                .is_on(settings.config.general.aux_code_show)
                .on_toggled(context.callback(Message::AuxCodeShow)),
        ),
        field(
            "碼刪空後保持輔碼",
            "刪空碼後 ; 仍在、候選全部回來，再按一次退格才退出輔碼；關掉則刪空即回拼音態。",
            ToggleSwitch::new()
                .is_on(settings.config.general.aux_code_keep_empty)
                .on_toggled(context.callback(Message::AuxCodeKeepEmpty)),
        ),
        TextBlock::new()
            .text("碼錶")
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        bundled_list(settings, context),
        user_list(settings, context),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                Button::new()
                    .on_click(context.message(Message::ImportCodeTable))
                    .content("匯入碼錶…"),
                note(
                    "接受 Rime 的 .dict.yaml（要有詞、碼兩列）與現成的 .qj。碼錶由你自己取得，青簡不隨包分發第三方形碼錶。",
                ),
            )),
        note(
            "檔案示例（詞與碼之間是製表符，另存為 my.dict.yaml 即可匯入）：\n---\nname: 我的碼錶\nversion: 1\n...\n開發\tkf\n開放\tkf\n鶴\thn",
        ),
        feedback(&settings.notice),
    ]);
    page("輔碼", body)
}

/// 錄製框裡敲進來的文本：合法就落盤，非法就紅字提示、保持原值、繼續等下一個鍵。
pub(crate) fn record_key(settings: &mut Settings, text: &str) {
    let Some(key) = text.chars().next() else {
        return;
    };
    let page_keys = settings.config.general.page_keys();
    if !is_valid_aux_code_key(key, page_keys) {
        settings.notice.fail(format!(
            "「{key}」不能當觸發鍵：要單個可見的符號，字母、數字、單引號與當前翻頁鍵（{}{}）不行。",
            page_keys.0, page_keys.1
        ));
        settings.recorder = settings.recorder.waiting();
        return;
    }
    settings.save("general", "aux_code_key", key.to_string());
    settings.notice.clear();
    settings.recorder = Recorder::Idle;
}

/// 挪進 `codes\removed`，不真刪（與詞庫一致）。
pub(crate) fn remove_table(settings: &Settings, stem: &str) {
    let dir = user_dir(settings);
    let Some((_, path)) = code_tables::list(&dir)
        .into_iter()
        .find(|(name, _)| name == stem)
    else {
        return;
    };
    let removed = dir.join("removed");
    if let Err(error) = std::fs::create_dir_all(&removed) {
        eprintln!("建 removed 目錄失敗: {error}");
        return;
    }
    if let Some(file_name) = path.file_name()
        && let Err(error) = std::fs::rename(&path, removed.join(file_name))
    {
        eprintln!("移除碼錶 {stem} 失敗: {error}");
    }
}

/// 檔案選擇器選一張碼錶，轉換後放進使用者碼錶目錄；統計進 note，失敗進紅字。
pub(crate) fn import(settings: &mut Settings) {
    let Some(source) = rfd::FileDialog::new()
        .add_filter("碼錶檔案", &["yaml", "yml", "qj"])
        .add_filter("所有檔案", &["*"])
        .set_title("匯入碼錶")
        .pick_file()
    else {
        return;
    };
    match import_aux_code_table(&source, &user_dir(settings)) {
        Ok(imported) => {
            let report = imported.report;
            settings.notice.succeed(format!(
                "已匯入碼錶「{}」：讀入 {} · 有碼 {} · 無碼 {} · 跳過 {}",
                imported.name, report.read, report.with_code, report.no_code, report.skipped
            ));
        }
        Err(DictionaryError::NoCodeEntries) => settings
            .notice
            .fail("這個檔案裡沒有識別到碼錶：要「詞 + 碼」兩列，碼只收 a–z。".to_owned()),
        Err(error) => settings.notice.fail(format!("匯入碼錶失敗：{error}")),
    }
}
