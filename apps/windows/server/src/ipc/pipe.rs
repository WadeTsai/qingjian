//! 命名管道傳輸：在 `\\.\pipe\qingjian` 上服務 DLL 客戶端，位元組模式，幀由協議 codec 切。
//!
//! 每個應用程序各開一條連線且失焦後連線仍在，所以不能序列服務（新聚焦的應用連不上會阻塞 UI 執行緒、
//! 觸發 TSF 看門狗）：後臺接受迴圈每來一個客戶端就新建例項、起一條執行緒；[`Router`] 不跨執行緒，
//! 留在呼叫執行緒跑工人迴圈，各連線經通道把訊息轉給它序列處理。

use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Instant;

use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_PIPE_CONNECTED, HANDLE};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows::core::{HRESULT, HSTRING};

use qingjian_platform::protocol::{ClientMessage, ServerMessage, read_message, write_message};

use super::Work;
use crate::dispatch::Router;

pub use qingjian_platform::protocol::DEFAULT_PIPE_NAME;

const BUFFER_SIZE: u32 = 64 * 1024;

/// 管道的 SDDL：放行 Everyone / ALL APPLICATION PACKAGES / ALL RESTRICTED APPLICATION PACKAGES，
/// 完整性標 Low。工作列搜尋、設定這類 AppContainer 程序在預設 DACL 下連不上。
const PIPE_SDDL: &str = "D:(A;;GA;;;WD)(A;;GA;;;AC)(A;;GA;;;S-1-15-2-2)S:(ML;;NW;;;LW)";

/// 在命名管道上服務多個客戶端。當前執行緒獨佔 [`Router`] 跑工人迴圈，正常不返回。
/// `work` 通道由呼叫方建（UI 執行緒也往裡投狀態條事件），這裡拿一份傳送端給各連線。
/// 第一個例項帶 `FILE_FLAG_FIRST_PIPE_INSTANCE`：已有一個 Server 在跑就建不出來，直接報錯退出（兩個 Server
/// 會各畫一條狀態條、各持一份狀態）。
pub fn serve_pipe(
    name: &str,
    router: &mut Router,
    sender: Sender<Work>,
    receiver: Receiver<Work>,
) -> io::Result<()> {
    let pipe = HSTRING::from(name);
    let first = match create_instance(&pipe, pipe_security_descriptor(), true) {
        Ok(first) => first,
        Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED.0 as i32) => {
            return Err(io::Error::other(
                "已有一个 qingjian-server 在运行（命名管道被占），本进程退出",
            ));
        }
        Err(error) => return Err(error),
    };
    thread::spawn(move || accept_loop(&pipe, first, sender));
    tracing::info!(pipe = name, "命名管道监听中");
    // 按 Router 的節拍來 tick：在等本地整句模型就幾十毫秒一次，否則一秒看一次配置檔案。
    // 到點時間是絕對的，不隨訊息重新計時——前臺程序裡的 DLL 隔幾百毫秒就問一次切模式（SyncMode），
    // 若每收一條訊息就重等一秒，tick 永遠到不了，熱載入與模型接入都會停擺。
    let mut due = Instant::now() + router.next_tick();
    loop {
        let now = Instant::now();
        if now >= due {
            router.tick();
            due = Instant::now() + router.next_tick();
            continue;
        }
        match receiver.recv_timeout(due - now) {
            Ok(Work::Client(message, reply)) => {
                let _ = reply.send(router.handle(message));
            }
            Ok(Work::Status(event)) => router.handle_status_event(event),
            Ok(Work::Click(target)) => router.handle_click(target),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
        // 處理完訊息節拍可能變短了（按鍵起了防抖）：到點時間只提前不推後
        due = due.min(Instant::now() + router.next_tick());
    }
    Ok(())
}

/// 先在建好的第一個例項上等客戶端，之後每建一個例項、等一個客戶端連上，就起一條執行緒服務它。建例項出錯才停。
fn accept_loop(name: &HSTRING, first: File, sender: Sender<Work>) {
    let descriptor = pipe_security_descriptor();
    let mut instance = Ok(first);
    loop {
        let stream = match instance.and_then(wait_client) {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(%error, "建管道实例失败，停止接受");
                break;
            }
        };
        let sender = sender.clone();
        thread::spawn(move || serve_connection(stream, sender));
        instance = create_instance(name, descriptor, false);
    }
}

/// 轉換失敗返回 null，退回預設 DACL。描述符建一次、隨程序存活。
fn pipe_security_descriptor() -> PSECURITY_DESCRIPTOR {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(PIPE_SDDL),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    };
    if let Err(error) = converted {
        tracing::warn!(%error, "构建管道安全描述符失败，退回默认 DACL（UWP 应用可能连不上）");
        return PSECURITY_DESCRIPTOR::default();
    }
    descriptor
}

/// 建一個例項。控制代碼交給 `File` 管：對端關閉時 std 把 `ERROR_BROKEN_PIPE` 當 EOF。
fn create_instance(
    name: &HSTRING,
    descriptor: PSECURITY_DESCRIPTOR,
    first: bool,
) -> io::Result<File> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    let attributes = (!descriptor.0.is_null()).then_some(&raw const attributes);
    let mut open_mode = PIPE_ACCESS_DUPLEX;
    if first {
        open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    let handle = unsafe {
        CreateNamedPipeW(
            name,
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER_SIZE,
            BUFFER_SIZE,
            0,
            attributes,
        )
    };
    if handle.is_invalid() {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_handle(handle.0) })
}

/// 阻塞等一個客戶端連上這個例項。
fn wait_client(stream: File) -> io::Result<File> {
    let handle = HANDLE(stream.as_raw_handle());
    // 客戶端在建例項與 ConnectNamedPipe 之間就連上了也算成功。
    if let Err(error) = unsafe { ConnectNamedPipe(handle, None) }
        && error.code() != HRESULT::from_win32(ERROR_PIPE_CONNECTED.0)
    {
        return Err(io::Error::other(error));
    }
    Ok(stream)
}

/// 服務一條連線：讀訊息 → 轉給工人執行緒 → 寫回，直到對端在幀邊界關閉或出錯。
fn serve_connection(mut stream: File, sender: Sender<Work>) {
    let (reply_sender, reply_receiver) = mpsc::channel::<Option<ServerMessage>>();
    loop {
        let message = match read_message::<_, ClientMessage>(&mut stream) {
            Ok(Some(message)) => message,
            Ok(None) => break,
            Err(error) => {
                tracing::warn!(%error, "客户端会话读出错");
                break;
            }
        };
        if sender
            .send(Work::Client(message, reply_sender.clone()))
            .is_err()
        {
            break;
        }
        match reply_receiver.recv() {
            Ok(Some(response)) => {
                if write_message(&mut stream, &response).is_err() {
                    break;
                }
            }
            Ok(None) => {}
            Err(_) => break,
        }
    }
    let _ = unsafe { DisconnectNamedPipe(HANDLE(stream.as_raw_handle())) };
    tracing::debug!("客户端断开");
}
