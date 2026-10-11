//! Unix socket 服務：連線內會話編號隔離、握手校驗、斷線回收和私有許可權。
use crate::dispatch::Router;
use qingjian_platform::protocol::{ClientMessage, ServerMessage};
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
mod connection;
mod session;
use connection::serve_connection;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::Instant;

/// 主執行緒請求佇列；容量有限，損壞客戶端不能無限佔用記憶體。
type Request = (serde_json::Value, mpsc::Sender<Option<serde_json::Value>>);
static STOP: AtomicBool = AtomicBool::new(false);
const MAX_CLIENT_CONNECTIONS: usize = 64;
static CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
/// 主執行緒在下一次空閒節拍退出，落盤由 Router::drop 完成。
pub fn request_shutdown() {
    STOP.store(true, Ordering::Relaxed);
}
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// 取得 Linux socket 路徑；沒有執行時目錄時使用按使用者隔離的私有臨時目錄。
pub fn socket_path() -> PathBuf {
    std::env::var_os("QINGJIAN_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("XDG_RUNTIME_DIR")
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(format!("/tmp/qingjian-{}", unsafe { libc::geteuid() }))
                })
                .join("qingjian.sock")
        })
}

/// 僅移除同用戶、確認拒絕連線的陳舊 socket；拒絕符號連結和不安全的父目錄。
pub fn bind_socket(path: &Path) -> io::Result<UnixListener> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| io::Error::other("socket requires an absolute parent directory"))?;
    if !path.is_absolute() {
        return Err(io::Error::other("socket path must be absolute"));
    }
    if !parent.exists() {
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }
    let owner = unsafe { libc::geteuid() };
    let metadata = std::fs::symlink_metadata(parent)?;
    if !metadata.is_dir() || metadata.uid() != owner || metadata.mode() & 0o022 != 0 {
        return Err(io::Error::other(
            "socket directory must be owned by this user and not writable by others",
        ));
    }
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.file_type().is_socket() || meta.uid() != owner {
                return Err(io::Error::other("unsafe socket path"));
            }
            match UnixStream::connect(path) {
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "server already running",
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
                    std::fs::remove_file(path)?
                }
                Err(error) => return Err(error),
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

pub fn serve_socket(path: impl AsRef<Path>, router: &mut Router) -> io::Result<()> {
    let listener = bind_socket(path.as_ref())?;
    let settings = router.display_settings();
    let (sender, receiver) = mpsc::sync_channel::<Request>(128);
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if CONNECTIONS.fetch_add(1, Ordering::Relaxed) >= MAX_CLIENT_CONNECTIONS {
                        CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
                        continue;
                    }
                    let sender = sender.clone();
                    let settings = settings.clone();
                    thread::spawn(move || {
                        serve_connection(stream, sender, settings);
                        CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
                    });
                }
                Err(error) => {
                    tracing::error!(%error, "接受客户端连接失败");
                    break;
                }
            }
        }
    });
    // 按 Router 的節拍來 tick：在等本地整句模型就幾十毫秒一次，否則一秒看一次要不要落盤學習。
    // 到點時間是絕對的，不隨訊息重新計時——組句期間外掛每 80 毫秒問一次，若每收一條訊息就重等，tick 永遠到不了。
    let mut due = Instant::now() + router.next_tick();
    while !STOP.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now >= due {
            router.tick();
            due = Instant::now() + router.next_tick();
            continue;
        }
        match receiver.recv_timeout(due - now) {
            Ok((message, reply)) => {
                let _ = reply.send(router.handle_linux(message));
                // 處理完訊息節拍可能變短了（按鍵起了防抖）：到點時間只提前不推後
                due = due.min(Instant::now() + router.next_tick());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
    std::fs::remove_file(path.as_ref())?;
    Ok(())
}

fn dispatch(sender: &SyncSender<Request>, message: ClientMessage) -> Option<ServerMessage> {
    let (reply, receiver) = mpsc::channel();
    sender
        .send((serde_json::to_value(message).ok()?, reply))
        .ok()?;
    serde_json::from_value(receiver.recv().ok().flatten()?).ok()
}

fn dispatch_json(
    sender: &SyncSender<Request>,
    message: serde_json::Value,
) -> Option<serde_json::Value> {
    let (reply, receiver) = mpsc::channel();
    sender.send((message, reply)).ok()?;
    receiver.recv().ok().flatten()
}

#[cfg(test)]
mod tests;
