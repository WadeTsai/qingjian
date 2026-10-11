//! Server 與 DLL 之間的傳輸。線上幀格式在 [`qingjian_platform::protocol`]（兩端共用），
//! 這層只提供雙工位元組流上的訊息迴圈（[`serve`]）與具體傳輸（命名管道 [`pipe`]）。

#[cfg(windows)]
pub mod pipe;
mod work;

pub use qingjian_platform::protocol::{CodecError, read_message, write_message};
pub use work::Work;

use std::io::{Read, Write};

use qingjian_platform::protocol::ClientMessage;

use crate::dispatch::Router;

/// 在一條已連上的雙工流上服務一個客戶端：讀訊息、交給 Router、寫回，直到對端在幀邊界關閉。
pub fn serve<S: Read + Write>(stream: &mut S, router: &mut Router) -> Result<(), CodecError> {
    while let Some(message) = read_message::<_, ClientMessage>(&mut *stream)? {
        if let Some(response) = router.handle(message) {
            write_message(&mut *stream, &response)?;
        }
    }
    Ok(())
}
