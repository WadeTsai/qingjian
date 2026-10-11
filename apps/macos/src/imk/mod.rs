//! InputMethodKit 這一側：輸入控制器、文本客戶端封裝、修飾鍵與 Secure Input 查詢。
//!
//! 只做兩件事：把系統輸入事件翻譯成 Engine 的呼叫，把 Engine 的結果交給候選視窗。
//! 排序、詞庫、翻譯邏輯一概不許出現在這裡。

mod client;
mod controller;
pub mod modifiers;
pub mod secure_input;

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub use client::TextClient;
pub use controller::{QingjianInputController, click};

/// 在 ObjC 執行時回撥的邊界攔住 panic。`define_class!` 生成的方法是系統直接調的，panic 穿過去整個程序就沒了，
/// 使用者正在打的字也跟著沒了。攔住後記一條錯誤日誌（位置與 backtrace 由 `main.rs` 裝的 panic hook 記），
/// 返回 `None` 讓呼叫方善後（見 [`recover_from_panic`]）。
pub fn catch_panic<R>(what: &'static str, f: impl FnOnce() -> R) -> Option<R> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => Some(value),
        Err(payload) => {
            tracing::error!(
                what,
                message = panic_message(&*payload),
                "回调里 panic，已拦下"
            );
            None
        }
    }
}

/// panic 之後的善後：把緩衝區裡的字母原樣交給應用（使用者敲過的鍵不能消失），再清掉引擎狀態、收起視窗。
/// 善後本身再 panic 就只能放棄，至少程序還活著。
pub fn recover_from_panic(client: Option<TextClient<'_>>) {
    let recovered = catch_unwind(AssertUnwindSafe(|| {
        let pending = crate::host::with(|h| {
            let text = h.engine.composition().text().to_owned();
            h.engine.clear();
            h.cancel_prediction();
            h.window.hide();
            text
        });
        if let (Some(client), Some(text)) = (client, pending)
            && !text.is_empty()
        {
            tracing::warn!(len = text.chars().count(), "panic 善后：缓冲区字母原样上屏");
            client.insert_text(&text);
        }
    }));
    if recovered.is_err() {
        tracing::error!("panic 善后又 panic，放弃本次会话状态");
    }
}

/// panic payload 裡的文字：`panic!("...")` 是 `&str` 或 `String`，別的型別給個佔位。
fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<非文字 payload>")
}
