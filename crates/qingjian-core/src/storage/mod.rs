//! 學習資料、配置這類小檔案的落盤方式：先寫同目錄的臨時檔案，落盤後再改名覆蓋。
//!
//! 輸入法程序隨時可能被 launchd 殺掉或崩潰；直接 `File::create` 覆蓋寫，中途斷掉就留下半個檔案，
//! 使用者攢的詞頻 / n-gram 整個報廢。改名在同一檔案系統上是原子的，所以任何時刻磁碟上要麼是舊檔案、要麼是新檔案。

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// 臨時檔案的字尾：與目標同目錄（改名不能跨檔案系統），帶程序號避免兩個程序互相覆蓋。
fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.tmp-{}", std::process::id()))
}

/// 原子地寫 `path`：`write` 往緩衝寫入端裡寫內容，寫完 flush + fsync，再改名覆蓋目標。
/// 中途任何一步失敗都不碰目標檔案，臨時檔案也清掉。
pub fn write_atomic(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    write_atomic_with(path, false, write)
}

/// 同 [`write_atomic`]，但檔案只有本使用者可讀寫（0600）：放金鑰的 `.env` 用。
pub fn write_atomic_private(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    write_atomic_with(path, true, write)
}

fn write_atomic_with(
    path: &Path,
    private: bool,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    let temporary = temporary_path(path);
    let result = (|| {
        let file = create(&temporary, private)?;
        let mut writer = BufWriter::new(file);
        write(&mut writer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// 建臨時檔案；`private` 時許可權 0600（非 Unix 平臺沒有這個概念，照常建）。
fn create(path: &Path, private: bool) -> io::Result<File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = private;
    options.open(path)
}

/// 把 `text` 原子地寫進 `path`。
pub fn write_atomic_str(path: &Path, text: &str) -> io::Result<()> {
    write_atomic(path, |writer| writer.write_all(text.as_bytes()))
}

/// 讀一個可能被寫壞的文本檔案：不存在返回 `None`；不是合法 UTF-8 的部分按替換字元讀進來，讓按行解析去跳過壞行，
/// 而不是整個檔案讀不了。其餘 io 錯誤照常返回。
pub fn read_text_lossy(path: &Path) -> io::Result<Option<String>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每個測試一個目錄：測試並行跑，共用目錄時一個測試的臨時檔案會被另一個測試的目錄掃描撞見
    fn scratch(test: &str, name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qingjian-storage-{}-{test}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn atomic_write_replaces_the_file_and_leaves_no_temporary() {
        let path = scratch("atomic", "atomic.tsv");
        write_atomic_str(&path, "old\n").unwrap();
        write_atomic_str(&path, "new\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
        let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(&format!("tmp-{}", std::process::id()))
            })
            .collect();
        assert!(leftovers.is_empty(), "临时文件没清：{leftovers:?}");
    }

    #[test]
    fn failed_write_keeps_the_old_file() {
        let path = scratch("kept", "kept.tsv");
        write_atomic_str(&path, "old\n").unwrap();
        let result = write_atomic(&path, |writer| {
            writer.write_all(b"half")?;
            Err(io::Error::other("boom"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "old\n");
        assert!(!temporary_path(&path).exists());
    }

    #[cfg(unix)]
    #[test]
    fn private_write_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let path = scratch("private", "private.env");
        write_atomic_private(&path, |writer| writer.write_all(b"KEY=1\n")).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn lossy_read_tolerates_broken_utf8_and_missing_files() {
        let path = scratch("lossy", "broken.tsv");
        fs::write(&path, b"\xe5\xa5\xbd\t1\n\xe5\xa5\t2\n").unwrap();
        let text = read_text_lossy(&path).unwrap().unwrap();
        assert!(text.starts_with("好\t1\n"));
        assert!(text.contains('\u{FFFD}'));
        assert!(
            read_text_lossy(&scratch("lossy", "missing.tsv"))
                .unwrap()
                .is_none()
        );
    }
}
