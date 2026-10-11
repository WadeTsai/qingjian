use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use qingjian_core::{InputLogEntry, InputLogger};
use serde::Serialize;

/// 輸入日誌落盤：每條一行 JSON（jsonl），追加寫，帶本機時間戳。只寫在這臺電腦的資料目錄裡。
///
/// 打不開檔案時退化成不記（記一次警告），輸入不受影響。
#[derive(Debug)]
pub struct InputLog {
    /// 檔案路徑。
    path: PathBuf,

    /// 開啟的寫入端；打不開為 `None`。
    file: Option<BufWriter<File>>,

    /// 自上次 flush 後寫了幾條（攢幾條再刷盤，別每次上屏都碰磁碟）。
    pending: usize,
}

/// 攢到這麼多條就刷一次盤；停用輸入法時也刷。
const FLUSH_EVERY: usize = 20;

/// 落盤的一行：時間戳 + 條目本身的欄位。
#[derive(Serialize)]
struct Line<'a> {
    /// 本機時間，帶時區偏移。
    t: String,

    #[serde(flatten)]
    entry: &'a InputLogEntry,
}

impl InputLog {
    /// 追加模式開啟（不存在就建）。
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let file = match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => Some(BufWriter::new(file)),
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "输入日志打不开，本次不记");
                None
            }
        };
        Self {
            path,
            file,
            pending: 0,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 清空日誌檔案（使用者在偏好設定裡點「清空」）。
    pub fn clear(path: &Path) -> std::io::Result<()> {
        File::create(path).map(|_| ())
    }

    /// 檔案裡有多少行（診斷資訊用）；檔案不存在算 0。
    pub fn line_count(path: &Path) -> usize {
        std::fs::read_to_string(path).map_or(0, |text| text.lines().count())
    }
}

impl InputLogger for InputLog {
    fn record(&mut self, entry: InputLogEntry) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let line = Line {
            t: jiff::Zoned::now()
                .strftime("%Y-%m-%dT%H:%M:%S%:z")
                .to_string(),
            entry: &entry,
        };
        let written = serde_json::to_string(&line)
            .map_err(std::io::Error::other)
            .and_then(|json| file.write_all(json.as_bytes()))
            .and_then(|()| file.write_all(b"\n"));
        if let Err(error) = written {
            tracing::warn!(%error, "输入日志写入失败，停止记录");
            self.file = None;
            return;
        }
        self.pending += 1;
        if self.pending >= FLUSH_EVERY {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if let Some(file) = self.file.as_mut()
            && let Err(error) = file.flush()
        {
            tracing::warn!(%error, "输入日志刷盘失败");
        }
        self.pending = 0;
    }

    fn is_enabled(&self) -> bool {
        self.file.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::{CommitEntry, InputSource};

    #[test]
    fn writes_one_json_line_per_entry_with_a_timestamp() {
        let dir = std::env::temp_dir().join(format!("qingjian-input-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("input-log.jsonl");
        let mut log = InputLog::open(&path);
        assert!(log.is_enabled());
        log.record(InputLogEntry::Commit(CommitEntry {
            id: 1,
            scope: "nihao".into(),
            keys: "nihao".into(),
            pinyin: "ni'hao".into(),
            corrected: false,
            text: "你好".into(),
            source: InputSource::Word,
            index: Some(0),
            top: vec!["你好".into(), "拟好".into()],
            scheme: String::new(),
            english: false,
            rescored: false,
            app: None,
            pages: 0,
            ms: 120,
        }));
        log.record(InputLogEntry::Retract {
            of: 1,
            text: "你好".into(),
            chosen: "拟好".into(),
        });
        log.flush();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"event\":\"commit\""));
        assert!(lines[0].contains("\"keys\":\"nihao\""));
        assert!(lines[0].starts_with("{\"t\":\""));
        assert!(lines[1].contains("\"event\":\"retract\""));
        assert!(lines[1].contains("\"of\":1"));
        assert_eq!(InputLog::line_count(&path), 2);
        InputLog::clear(&path).unwrap();
        assert_eq!(InputLog::line_count(&path), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
