//! 線上幀格式：4 位元組小端長度字首 + JSON 訊息體。Server 與 TSF DLL 兩端共用這套讀寫，
//! 所以放在平臺層而不是某個 app 裡。傳輸本身（命名管道等）在 app 側，這裡只切幀。

use std::io::{self, Read, Write};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// 預設命名管道名。Server 在這上面監聽，DLL 用同名連上。放這裡讓兩端共享同一個字面量。
pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\qingjian";

/// 單幀上限，擋住壞長度字首導致的巨量分配。
const MAX_FRAME: u32 = 16 * 1024 * 1024;

/// 編解碼錯誤。
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("frame too large: {0} bytes")]
    TooLarge(usize),
}

/// 寫一幀：長度字首 + JSON，然後 flush。
pub fn write_message<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<(), CodecError> {
    let body = serde_json::to_vec(value)?;
    let len = u32::try_from(body.len()).map_err(|_| CodecError::TooLarge(body.len()))?;
    if len > MAX_FRAME {
        return Err(CodecError::TooLarge(body.len()));
    }
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(&body)?;
    writer.flush()?;
    Ok(())
}

/// 讀一幀。對端在幀邊界乾淨關閉（讀長度字首時就是 EOF）返回 `Ok(None)`；
/// 讀到一半斷開算錯誤（`UnexpectedEof`）。
pub fn read_message<R: Read, T: DeserializeOwned>(reader: &mut R) -> Result<Option<T>, CodecError> {
    let mut len_buf = [0u8; 4];
    if !read_fully_or_eof(reader, &mut len_buf)? {
        return Ok(None);
    }
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_FRAME {
        return Err(CodecError::TooLarge(len as usize));
    }
    let mut body = vec![0u8; len as usize];
    reader.read_exact(&mut body)?;
    Ok(Some(serde_json::from_slice(&body)?))
}

/// 填滿 `buf`。開頭就 EOF（一個位元組沒讀到）返回 `Ok(false)`（乾淨關閉）；
/// 讀到一半 EOF 返回 `UnexpectedEof`；填滿返回 `Ok(true)`。
fn read_fully_or_eof<R: Read>(reader: &mut R, buf: &mut [u8]) -> io::Result<bool> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..])? {
            0 if filled == 0 => return Ok(false),
            0 => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "connection closed mid-frame",
                ));
            }
            read => filled += read,
        }
    }
    Ok(true)
}
