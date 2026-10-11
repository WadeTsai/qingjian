//! JSONL 結果檔案：逐行追加、啟動時讀回已完成的詞、匯出成釋義表。

use std::collections::{BTreeMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::entry::{EnglishGlossEntry, GlossEntry, PinyinEntry};
use crate::error::GlossError;

/// JSONL 裡的一行：按詞去重續跑。
pub trait Keyed: Serialize + DeserializeOwned {
    /// 這條結果對應的詞。
    fn key(&self) -> &str;
}

impl Keyed for GlossEntry {
    fn key(&self) -> &str {
        &self.word
    }
}

impl Keyed for PinyinEntry {
    fn key(&self) -> &str {
        &self.word
    }
}

impl Keyed for EnglishGlossEntry {
    fn key(&self) -> &str {
        &self.word
    }
}

/// 開啟著的結果檔案。
pub struct Store<T> {
    /// 已有的詞。
    done: HashSet<String>,

    /// 追加寫入端。
    file: Mutex<File>,

    /// 條目型別。
    _entry: PhantomData<T>,
}

impl<T: Keyed> Store<T> {
    /// 開啟（沒有就建）並讀回已有的條目。
    pub fn open(path: &Path) -> Result<Self, GlossError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let done: HashSet<String> = if path.exists() {
            read_entries::<T>(path)?
                .into_iter()
                .map(|e| e.key().to_owned())
                .collect()
        } else {
            HashSet::new()
        };
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            done,
            file: Mutex::new(file),
            _entry: PhantomData,
        })
    }

    pub fn contains(&self, word: &str) -> bool {
        self.done.contains(word)
    }

    pub fn len(&self) -> usize {
        self.done.len()
    }

    /// 追加一批條目並落盤。
    pub fn append(&self, entries: &[T]) -> Result<(), GlossError> {
        let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
        for entry in entries {
            serde_json::to_writer(&mut *file, entry)?;
            file.write_all(b"\n")?;
        }
        file.flush()?;
        Ok(())
    }
}

/// 讀全部條目，壞行跳過並記一條警告。
pub fn read_entries<T: Keyed>(path: &Path) -> Result<Vec<T>, GlossError> {
    let reader = BufReader::new(File::open(path)?);
    let mut entries = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(&line) {
            Ok(entry) => entries.push(entry),
            Err(error) => tracing::warn!(line = index + 1, %error, "跳过坏行"),
        }
    }
    Ok(entries)
}

/// 匯出成輸入法載入的學習語言表：`詞\t詞性 譯詞\t詞性 譯詞`，日文譯詞後面接 `|假名`。同一個詞以最後一條為準，按詞排序。
pub fn export(input: &Path, out_dir: &Path) -> Result<(), GlossError> {
    let mut latest: BTreeMap<String, GlossEntry> = BTreeMap::new();
    for entry in read_entries::<GlossEntry>(input)? {
        latest.insert(entry.word.clone(), entry);
    }
    std::fs::create_dir_all(out_dir)?;
    let mut english = Vec::new();
    let mut japanese = Vec::new();
    let mut vietnamese = Vec::new();
    let (mut en_count, mut ja_count, mut vi_count) = (0, 0, 0);
    for entry in latest.values() {
        let pos = entry
            .pos
            .as_deref()
            .map(|p| format!("{p} "))
            .unwrap_or_default();
        if !entry.en.is_empty() {
            write!(english, "{}", entry.word)?;
            for sense in &entry.en {
                write!(english, "\t{pos}{sense}")?;
            }
            writeln!(english)?;
            en_count += 1;
        }
        if !entry.ja.is_empty() {
            write!(japanese, "{}", entry.word)?;
            for sense in &entry.ja {
                match &sense.reading {
                    Some(reading) => write!(japanese, "\t{pos}{}|{reading}", sense.text)?,
                    None => write!(japanese, "\t{pos}{}", sense.text)?,
                }
            }
            writeln!(japanese)?;
            ja_count += 1;
        }
        if !entry.vi.is_empty() {
            write!(vietnamese, "{}", entry.word)?;
            for sense in &entry.vi {
                write!(vietnamese, "\t{pos}{sense}")?;
            }
            writeln!(vietnamese)?;
            vi_count += 1;
        }
    }
    write_if_non_empty(
        out_dir,
        "glossary-en.tsv",
        "# 由 qingjian-gloss-gen 生成（LLM）。词\t[词性. ]译词\t[词性. ]译词",
        &english,
    )?;
    write_if_non_empty(
        out_dir,
        "glossary-ja.tsv",
        "# 由 qingjian-gloss-gen 生成（LLM）。词\t[词性. ]译词|假名\t[词性. ]译词|假名",
        &japanese,
    )?;
    write_if_non_empty(
        out_dir,
        "glossary-vi.tsv",
        "# 由 qingjian-gloss-gen 生成（LLM）。词\t[词性. ]译词\t[词性. ]译词",
        &vietnamese,
    )?;
    tracing::info!(
        words = latest.len(),
        english = en_count,
        japanese = ja_count,
        vietnamese = vi_count,
        out_dir = %out_dir.display(),
        "导出完成"
    );
    Ok(())
}

fn write_if_non_empty(
    out_dir: &Path,
    name: &str,
    header: &str,
    body: &[u8],
) -> Result<(), GlossError> {
    if body.is_empty() {
        return Ok(());
    }
    let path = out_dir.join(name);
    let mut out = BufWriter::new(File::create(path)?);
    writeln!(out, "{header}")?;
    out.write_all(body)?;
    out.flush()?;
    Ok(())
}

/// 英→中釋義匯出成 `glossary-zh.tsv`：`詞\t詞性. 釋義\t…`，鍵是小寫（英文候選按敲的大小寫顯示，查表時統一小寫）。
/// 同一個詞以最後一條為準，按詞排序。
pub fn export_english(input: &Path, out_dir: &Path) -> Result<(), GlossError> {
    let mut latest: BTreeMap<String, EnglishGlossEntry> = BTreeMap::new();
    for entry in read_entries::<EnglishGlossEntry>(input)? {
        latest.insert(entry.word.to_ascii_lowercase(), entry);
    }
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join("glossary-zh.tsv");
    let mut out = BufWriter::new(File::create(&path)?);
    writeln!(
        out,
        "# 由 qingjian-gloss-gen english 生成（LLM）：英文词的中文释义。词（小写）\t[词性. ]释义\t[词性. ]释义"
    )?;
    let mut count = 0;
    for (key, entry) in &latest {
        if entry.zh.is_empty() {
            continue;
        }
        let pos = entry
            .pos
            .as_deref()
            .map(|p| format!("{p} "))
            .unwrap_or_default();
        write!(out, "{key}")?;
        for sense in &entry.zh {
            write!(out, "\t{pos}{sense}")?;
        }
        writeln!(out)?;
        count += 1;
    }
    out.flush()?;
    tracing::info!(words = latest.len(), exported = count, out = %path.display(), "英→中释义导出完成");
    Ok(())
}
