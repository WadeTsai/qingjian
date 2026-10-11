use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::FILE_PREFIX;

/// 按天分檔案的日誌寫入端：日期變了換檔案，檔案在磁碟上被刪了就重開（`nlink == 0` 說明當前描述符指向的
/// inode 已經從目錄裡摘掉）。每次寫之前 fstat 一次，代價可以忽略，且寫入本來就在 `non_blocking` 的後臺執行緒。
pub struct LogFile {
    /// 日誌目錄。
    dir: PathBuf,

    /// 當前開啟的檔案及其日期；打不開時為 `None`，下次寫再試。
    current: Option<(jiff::civil::Date, File)>,
}

impl LogFile {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, current: None }
    }

    /// 某一天的日誌檔案路徑。
    pub fn path_for(dir: &Path, date: jiff::civil::Date) -> PathBuf {
        dir.join(format!("{FILE_PREFIX}.{date}"))
    }

    /// 拿到今天的檔案：日期變了、還沒開啟、或者檔案已被刪掉時重新開啟（追加模式）。
    fn file(&mut self) -> io::Result<&mut File> {
        let today = jiff::Zoned::now().date();
        let stale = match &self.current {
            Some((date, file)) => *date != today || file.metadata().is_ok_and(|m| m.nlink() == 0),
            None => true,
        };
        if stale {
            std::fs::create_dir_all(&self.dir)?;
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(Self::path_for(&self.dir, today))?;
            self.current = Some((today, file));
        }
        Ok(&mut self.current.as_mut().expect("just opened").1)
    }
}

impl Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.current {
            Some((_, file)) => file.flush(),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reopens_after_the_file_is_deleted() {
        let dir = std::env::temp_dir().join("qingjian-log-file-test");
        let _ = std::fs::remove_dir_all(&dir);
        let mut log = LogFile::new(dir.clone());
        log.write_all(b"one\n").unwrap();
        let path = LogFile::path_for(&dir, jiff::Zoned::now().date());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "one\n");
        std::fs::remove_file(&path).unwrap();
        log.write_all(b"two\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
