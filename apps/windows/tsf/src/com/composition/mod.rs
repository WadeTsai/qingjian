//! 組句 preedit：把 Server 回的拼音行經 TSF 組句（[`ITfComposition`]）顯示在文件游標處（對應 macOS 的內聯 marked text）。
//! 只放最樸素的一行拼音；富樣式的拼音行在候選窗口裡另畫。所有寫操作都在非同步讀寫編輯會話裡做（理由見 `edit_session`）。

mod shared;
mod sink;

use std::mem::ManuallyDrop;
use std::rc::Rc;

use windows::Win32::UI::TextServices::{
    INSERT_TEXT_AT_SELECTION_FLAGS, ITfComposition, ITfCompositionSink, ITfContext,
    ITfContextComposition, ITfInsertAtSelection, ITfRange, TF_AE_END, TF_ANCHOR_END,
    TF_IAS_QUERYONLY, TF_SELECTION, TF_SELECTIONSTYLE,
};
use windows::core::{Interface, Result};

use qingjian_platform::protocol::{Frame, PreeditKind};

pub(crate) use self::shared::Shared;
use self::sink::CompositionSink;
use super::edit::{InputContext, anchor_rect, caret_rect, input_context};
use super::service::SharedClient;

/// 內聯要顯示的拼音行（跳過被糾錯劃掉的原字母）；空串表示沒有組句內容。
pub(crate) fn preedit_string(frame: &Frame) -> String {
    frame
        .preedit
        .iter()
        .filter(|segment| segment.kind != PreeditKind::Corrected)
        .map(|segment| segment.text.as_str())
        .collect()
}

/// 在編輯會話回撥（持寫鎖 `ec`）裡調：先落定 `commit`，再按 `preedit` 起 / 改 / 收組句，最後把游標位置報給 Server。
/// 新起一段組句時順手判輸入框私密不私密（變了就告訴 Server）、把游標前的文字送給 Server（本地整句模型的前文）。
pub(crate) fn apply(
    shared: &Rc<Shared>,
    engine: &SharedClient,
    context: &ITfContext,
    ec: u32,
    commit: Option<&str>,
    preedit: &str,
) -> Result<()> {
    if let Some(text) = commit {
        commit_text(shared, context, ec, text)?;
    }
    // 一段組句裡只問一次輸入框狀態。行內模式看組句剛起；`preedit = window` 模式應用里根本沒有組句，
    // 得另用一個標記，否則每敲一鍵都要重讀一遍游標前文、重報一次私密狀態。
    let report_input = !shared.has_composition() && !shared.context_reported();
    if report_input {
        shared.set_context_reported(true);
    }
    let input = report_input.then(|| input_context(context, ec));
    if preedit.is_empty() {
        end_composition(shared, ec)?;
    } else {
        update_preedit(shared, context, ec, preedit)?;
    }
    if let Some(InputContext { private, before }) = input {
        report_privacy(engine, private);
        if let Some(before) = before {
            report_surrounding(engine, before);
        }
    }
    report_caret(shared, engine, context, ec);
    Ok(())
}

/// 告訴 Server 輸入框私密與否（客戶端只在變了時真發）；引擎正被別處藉著（罕見）就算了，下段組句再報。
fn report_privacy(engine: &SharedClient, private: bool) {
    if let Ok(mut guard) = engine.try_borrow_mut()
        && let Some(client) = guard.as_mut()
        && let Err(error) = client.set_private(private)
    {
        super::log::log(&format!("報私密狀態失敗: {error}"));
    }
}

/// 把游標前文送給 Server；引擎正被別處藉著（罕見）就算了，Server 退回會話歷史。
fn report_surrounding(engine: &SharedClient, before: String) {
    let chars = before.chars().count();
    if let Ok(mut guard) = engine.try_borrow_mut()
        && let Some(client) = guard.as_mut()
    {
        match client.surrounding(before) {
            Ok(()) => super::log::log(&format!("送游標前文 {chars} 字")),
            Err(error) => super::log::log(&format!("送游標前文失敗: {error}")),
        }
    }
}

/// 組句進行中才報位置；組句已收 Server 會按空幀 / `Commit` 自行收視窗。
fn report_caret(shared: &Shared, engine: &SharedClient, context: &ITfContext, ec: u32) {
    let rect = match shared.composition() {
        Some(composition) => {
            let Ok(range) = (unsafe { composition.GetRange() }) else {
                return;
            };
            anchor_rect(context, ec, &range)
        }
        // 「只在候選視窗」模式應用裡不放行內拼音：沒有組句範圍可量，量插入點。
        None if shared.composing() => caret_rect(context, ec),
        None => return,
    };
    // 引擎正被別處藉著（罕見）就跳過這拍，Server 保持上次位置。
    if let Ok(mut guard) = engine.try_borrow_mut()
        && let Some(client) = guard.as_mut()
        && let Err(error) = client.position_candidates(rect)
    {
        super::log::log(&format!("上報候選視窗位置失敗: {error}"));
    }
}

/// 有組句就把組句範圍替換成 `text` 再結束組句，否則在選區插入。
fn commit_text(shared: &Shared, context: &ITfContext, ec: u32, text: &str) -> Result<()> {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    match shared.composition() {
        Some(composition) => {
            let range = unsafe { composition.GetRange()? };
            unsafe { range.SetText(ec, 0, &utf16)? };
            move_selection_to_end(context, ec, &range)?;
            unsafe { composition.EndComposition(ec)? };
            shared.set_composition(None);
        }
        None => {
            let insert: ITfInsertAtSelection = context.cast()?;
            // 標誌不能用 NOQUERY：它不回傳 range，windows-rs 會把 NULL 當失敗。
            let range = unsafe {
                insert.InsertTextAtSelection(ec, INSERT_TEXT_AT_SELECTION_FLAGS(0), &utf16)?
            };
            // 不移游標的話下一次插入又落在原處，字會從右往左堆。
            move_selection_to_end(context, ec, &range)?;
        }
    }
    Ok(())
}

fn update_preedit(shared: &Rc<Shared>, context: &ITfContext, ec: u32, preedit: &str) -> Result<()> {
    let composition = match shared.composition() {
        Some(composition) => composition,
        None => start_composition(shared, context, ec)?,
    };
    let utf16: Vec<u16> = preedit.encode_utf16().collect();
    let range = unsafe { composition.GetRange()? };
    unsafe { range.SetText(ec, 0, &utf16)? };
    super::display_attribute::mark(context, ec, &range);
    move_selection_to_end(context, ec, &range)
}

/// 在當前選區處起一個空組句；組句 sink 交給框架持有。
fn start_composition(shared: &Rc<Shared>, context: &ITfContext, ec: u32) -> Result<ITfComposition> {
    let insert: ITfInsertAtSelection = context.cast()?;
    let range = unsafe { insert.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])? };
    let context_composition: ITfContextComposition = context.cast()?;
    let sink: ITfCompositionSink = CompositionSink::new(shared.clone()).into();
    let composition = unsafe { context_composition.StartComposition(ec, &range, &sink)? };
    shared.set_composition(Some(composition.clone()));
    Ok(composition)
}

/// 清空組句文本再結束，避免殘留拼音。
fn end_composition(shared: &Shared, ec: u32) -> Result<()> {
    if let Some(composition) = shared.take_composition() {
        let range = unsafe { composition.GetRange()? };
        unsafe { range.SetText(ec, 0, &[])? };
        unsafe { composition.EndComposition(ec)? };
    }
    Ok(())
}

fn move_selection_to_end(context: &ITfContext, ec: u32, range: &ITfRange) -> Result<()> {
    let end = unsafe { range.Clone()? };
    unsafe { end.Collapse(ec, TF_ANCHOR_END)? };
    let selection = TF_SELECTION {
        range: ManuallyDrop::new(Some(end)),
        style: TF_SELECTIONSTYLE {
            ase: TF_AE_END,
            fInterimChar: false.into(),
        },
    };
    // SetSelection 不接管 range 的所有權，之後手動釋放。
    let result = unsafe { context.SetSelection(ec, std::slice::from_ref(&selection)) };
    drop(ManuallyDrop::into_inner(selection.range));
    result
}
