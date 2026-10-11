//! `cfg(windows)`：開啟 Server 的命名管道，得到一條雙工流交給 [`EngineClient`](super::EngineClient)。

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;

use windows::Win32::Foundation::ERROR_PIPE_BUSY;
use windows::Win32::System::Pipes::WaitNamedPipeW;
use windows::core::HSTRING;

use qingjian_platform::protocol::DEFAULT_PIPE_NAME;

/// 連好的命名管道。對端關閉時讀到 EOF；`flush` 是空操作（管道上 `FlushFileBuffers` 會阻塞到對端讀完）。
pub type PipeStream = File;

/// 例項都被佔著時等一個可用例項的超時（毫秒）與重試次數。要短：這裡在應用 UI 執行緒上，等久了 TSF 看門狗會切走輸入法。
const BUSY_WAIT_MS: u32 = 300;
const BUSY_RETRIES: u32 = 1;

pub fn connect_default() -> io::Result<PipeStream> {
    connect(DEFAULT_PIPE_NAME)
}

pub fn connect(name: &str) -> io::Result<PipeStream> {
    let mut attempts = 0;
    loop {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(name)
        {
            Err(error)
                if error.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32)
                    && attempts < BUSY_RETRIES =>
            {
                attempts += 1;
                let _ = unsafe { WaitNamedPipeW(&HSTRING::from(name), BUSY_WAIT_MS) };
            }
            result => return result,
        }
    }
}
