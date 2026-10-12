//! 輔碼碼錶：隨包的（隨包根 `codes/`）與使用者目錄 `codes/` 下的 `.qj`，按 `[aux_code] disabled` 過濾，
//! 載入後一起接到 Engine 上。與 `extra_dictionaries` 同構，只是裝的是碼錶。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use qingjian_dictionary::{AuxCodeLookup, AuxCodeTable};

use crate::AuxCodeConfig;

/// 碼錶的副檔名：只認打包過的 `.qj`（TSV 在匯入時就轉掉了）。
const EXTENSION: &str = "qj";

/// 列出目錄裡的碼錶檔案（按檔名排序），返回 (檔名不含副檔名, 路徑)。目錄不存在就是空。
pub fn list(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter_map(|path| {
            (path.extension()?.to_str()? == EXTENSION).then(|| {
                let stem = path.file_stem()?.to_str()?.to_owned();
                Some((stem, path))
            })?
        })
        .collect();
    files.sort();
    files
}

/// 可載入碼錶的快照：用於發現新增、移除與同名更新，不讀取碼錶正文。
/// 逐檔案比 (mtime, 長度)：半拷進來的 `.qj`（寫完長度才穩定）與就地改寫都能被發現。
pub fn snapshot(dir: &Path) -> Vec<(PathBuf, Option<std::time::SystemTime>, u64)> {
    list(dir)
        .into_iter()
        .filter_map(|(_, path)| {
            let metadata = std::fs::metadata(&path).ok()?;
            Some((path, metadata.modified().ok(), metadata.len()))
        })
        .collect()
}

/// 載入沒被關掉的碼錶：先隨包，再使用者目錄。壞檔案只記日誌、跳過：一張碼錶壞了不能拖垮輸入法。
pub fn load(
    bundled_dir: Option<&Path>,
    user_dir: Option<&Path>,
    config: &AuxCodeConfig,
) -> Vec<Arc<dyn AuxCodeLookup>> {
    let mut loaded: Vec<Arc<dyn AuxCodeLookup>> = Vec::new();
    for dir in [bundled_dir, user_dir].into_iter().flatten() {
        for (stem, path) in list(dir) {
            if !config.is_enabled(&stem) {
                tracing::debug!(name = %stem, "碼錶已關閉，跳過");
                continue;
            }
            match AuxCodeTable::open(&path) {
                Ok(table) => {
                    tracing::info!(
                        name = %table.metadata().map_or(stem.as_str(), |m| m.name.as_str()),
                        file = %path.display(),
                        entries = table.len(),
                        words = table.word_count(),
                        license = %table.metadata().map_or("", |m| m.license.as_str()),
                        "碼錶已載入"
                    );
                    loaded.push(Arc::new(table));
                }
                Err(error) => tracing::warn!(file = %path.display(), %error, "碼錶載入失敗，跳過"),
            }
        }
    }
    loaded
}
