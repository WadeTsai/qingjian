use std::sync::mpsc::Sender;

use qingjian_platform::protocol::{ClientMessage, ServerMessage};
use qingjian_render::HitTarget;

use crate::dispatch::StatusEvent;

/// 工人執行緒（獨佔 [`crate::dispatch::Router`]）的一件活：DLL 的一條訊息，或狀態條 / 候選視窗上的一次點選。
pub enum Work {
    /// 某條連線收到的訊息 + 回結果的通道（`None` 表示不用回話）。
    Client(ClientMessage, Sender<Option<ServerMessage>>),

    /// UI 執行緒發來的狀態條操作。
    Status(StatusEvent),

    /// UI 執行緒發來的候選視窗點選。
    Click(HitTarget),
}
