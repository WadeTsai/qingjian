//! 偏好設定各頁共用的控制元件搭建函式：標籤、說明小字、成行的彈出選單 / 文本框 / 錄製按鈕，
//! 以及把控制元件接到 [`PreferencesTarget`] 的 `changed:` 上。頁面檔案只描述「放什麼」，不重複這些細節。

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSButton, NSColor, NSControl, NSControlStateValueOff, NSControlStateValueOn, NSFont,
    NSPopUpButton, NSSecureTextField, NSTextAlignment, NSTextField,
};
use objc2_foundation::{NSArray, NSRect, NSString};
use qingjian_core::Language;

use super::key_recorder::KeyRecorder;
use super::layout::{CONTROL_X, LABEL_WIDTH, Layout, PAGE_PADDING, ROW_HEIGHT};
use super::setting::Setting;
use super::target::PreferencesTarget;

/// 說明小字一行的高度。
pub(super) const NOTE_HEIGHT: f64 = 15.0;

/// 說明小字按多少畫素一個字估算折行（11 號字，中文約 11 px，估得寬一點寧可多留一行）。
const NOTE_CHAR_WIDTH: f64 = 11.5;

/// 分組之間的留白。
pub(super) const GROUP_GAP: f64 = 14.0;

pub(super) fn language_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "中文",
        Language::English => "英语",
        Language::Japanese => "日语",
        Language::Spanish => "西班牙语",
        Language::Vietnamese => "越南语",
    }
}

/// `,.` → `,  .`。
pub(super) fn page_keys_label(pair: &str) -> String {
    let mut chars = pair.chars();
    match (chars.next(), chars.next()) {
        (Some(previous), Some(next)) => format!("{previous}   {next}"),
        _ => pair.to_owned(),
    }
}

/// 換掉彈出選單的全部選項（列表會變的，如主題）。
pub(super) fn set_items(popup: &NSPopUpButton, titles: &[String]) {
    popup.removeAllItems();
    let items: Vec<Retained<NSString>> = titles.iter().map(|t| NSString::from_str(t)).collect();
    popup.addItemsWithTitles(&NSArray::from_retained_slice(&items));
}

pub(super) fn select(popup: &NSPopUpButton, index: Option<usize>) {
    if let Some(index) = index {
        popup.selectItemAtIndex(index as isize);
    }
}

pub(super) fn set_checked(button: &NSButton, on: bool) {
    button.setState(if on {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
}

/// 掛 target / action / tag：所有控制元件都發 `changed:`，靠 tag 區分。
fn wire(control: &NSControl, setting: Setting, target: &PreferencesTarget) {
    // SAFETY: 選擇器與 PreferencesTarget 上定義的 `changed:` 一致，簽名 (id) -> void
    unsafe {
        control.setTarget(Some(target));
        control.setAction(Some(sel!(changed:)));
    }
    control.setTag(setting.tag());
}

pub(super) fn small_label(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFont(Some(&NSFont::systemFontOfSize(11.0)));
    label.setTextColor(Some(&NSColor::secondaryLabelColor()));
    label
}

/// 控制元件下方的說明小字，與控制元件列對齊，放不下就折行（按字數估行數，寧可多留一行）。
pub(super) fn note(layout: &mut Layout, mtm: MainThreadMarker, text: &str) {
    note_at(layout, mtm, text, CONTROL_X, layout.control_width());
}

/// 整行寬的說明小字（勾選框、按鈕下面用）。
pub(super) fn note_full(layout: &mut Layout, mtm: MainThreadMarker, text: &str) {
    note_at(layout, mtm, text, PAGE_PADDING, layout.inner_width());
}

fn note_at(layout: &mut Layout, mtm: MainThreadMarker, text: &str, x: f64, width: f64) {
    let label = small_label(mtm, text);
    label.setUsesSingleLineMode(false);
    if let Some(cell) = label.cell() {
        cell.setWraps(true);
    }
    let estimated = text.chars().count() as f64 * NOTE_CHAR_WIDTH;
    let lines = (estimated / width).ceil().max(1.0);
    let height = NOTE_HEIGHT * lines;
    layout.place(&label, x, width, height);
    layout.next_row(height);
}

/// 勾選框獨佔一行，從標題列起始處擺（勾選框自帶標題，不用左列標題）。
pub(super) fn row_checkbox(layout: &mut Layout, button: &NSButton) {
    layout.place(button, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT);
    layout.next_row(ROW_HEIGHT);
}

/// 標題列，右對齊貼著控制元件。
pub(super) fn caption(mtm: MainThreadMarker, text: &str) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setAlignment(NSTextAlignment::Right);
    label
}

/// 一行「標題 + 控制元件」，控制元件佔滿控制元件列。
pub(super) fn row_control(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    title: &str,
    control: &NSControl,
) {
    let label = caption(mtm, title);
    layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
    layout.place(control, CONTROL_X, layout.control_width(), ROW_HEIGHT);
    layout.next_row(ROW_HEIGHT);
}

pub(super) fn row_popup(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    title: &str,
    titles: &[String],
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSPopUpButton> {
    let popup = NSPopUpButton::initWithFrame_pullsDown(mtm.alloc(), NSRect::ZERO, false);
    let items: Vec<Retained<NSString>> = titles.iter().map(|t| NSString::from_str(t)).collect();
    popup.addItemsWithTitles(&NSArray::from_retained_slice(&items));
    wire(&popup, setting, target);
    let label = caption(mtm, title);
    layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
    layout.place(
        &popup,
        CONTROL_X,
        layout.control_width().min(200.0),
        ROW_HEIGHT,
    );
    layout.next_row(ROW_HEIGHT);
    popup
}

/// 一行「標題 + 快捷鍵錄製按鈕」。
pub(super) fn row_recorder(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    title: &str,
    setting: Setting,
    modifiers_only: bool,
    target: &PreferencesTarget,
) -> Retained<KeyRecorder> {
    let recorder = KeyRecorder::new(mtm, modifiers_only);
    wire(&recorder, setting, target);
    let label = caption(mtm, title);
    layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
    layout.place(&recorder, CONTROL_X, 160.0, ROW_HEIGHT);
    layout.next_row(ROW_HEIGHT);
    recorder
}

pub(super) fn checkbox(
    mtm: MainThreadMarker,
    title: &str,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSButton> {
    // SAFETY: target / action 與 `wire` 相同
    let button = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(sel!(changed:)),
            mtm,
        )
    };
    button.setTag(setting.tag());
    button
}

pub(super) fn button(
    mtm: MainThreadMarker,
    title: &str,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSButton> {
    // SAFETY: 同上
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(sel!(changed:)),
            mtm,
        )
    };
    button.setTag(setting.tag());
    button
}

/// 可編輯單行文本框：回車或失焦時發 action。
pub(super) fn text_field(
    mtm: MainThreadMarker,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSTextField> {
    let field = NSTextField::initWithFrame(mtm.alloc(), NSRect::ZERO);
    editable(&field, setting, target);
    field
}

pub(super) fn secure_field(
    mtm: MainThreadMarker,
    setting: Setting,
    target: &PreferencesTarget,
) -> Retained<NSSecureTextField> {
    let field = NSSecureTextField::initWithFrame(mtm.alloc(), NSRect::ZERO);
    editable(&field, setting, target);
    field
}

fn editable(field: &NSTextField, setting: Setting, target: &PreferencesTarget) {
    field.setBezeled(true);
    field.setEditable(true);
    field.setSelectable(true);
    field.setDrawsBackground(true);
    field.setUsesSingleLineMode(true);
    if let Some(cell) = field.cell() {
        cell.setSendsActionOnEndEditing(true);
    }
    wire(field, setting, target);
}
