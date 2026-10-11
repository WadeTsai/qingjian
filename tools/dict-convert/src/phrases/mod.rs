//! 短語層：常用詞表是詞典詞頭，不收 我的 / 好的 / 不知道 / 有沒有 這類人會整塊打出來的組合，輸入法裡它們卻是最常打的。
//! 掃兩遍語料：第一遍按詞庫分詞數相鄰兩詞，第二遍只數兩段二元都夠頻的相鄰三詞；取出現夠多、邊界像話的組合，讀音直接由成分詞的讀音拼出（不用再標多音字），寫成 `phrases.tsv`（`詞\t次數\t拼音`），
//! 由 `lexicon --extra-words` 併入基礎詞庫。
//!
//! 次數按語料檔案分開數，短語要在**對話語料**（`--dialogue`，人打字的樣子）裡就夠頻：維基裡的模板句（一個市鎮、面積為）
//! 次數再高也不是人會打的。只按次數與邊界規則（[`rules`]）篩，不用點互資訊：虛詞組合的 PMI 天然低，而它們正是要收的。
//! 結果要人工過一遍再拷進 `assets/lexicon/`。
//!
//! 分詞用的詞庫必須是**還沒併入短語**的：併入後 我的 是一個詞，相鄰詞對裡就沒有 我 + 的 了。重跑時給 `--refresh <上次的 phrases.tsv>`，
//! 裡面的詞會先從分詞詞表裡摘掉。

mod rules;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::bigram::{Vocabulary, is_han};
use crate::error::ConvertError;

/// `phrases` 子命令的引數。
pub struct PhraseOptions {
    /// 分詞與成分讀音用的詞庫（同目錄 `dicts/` 一併讀）。
    pub dict: PathBuf,

    /// 上一次的短語表：裡面的詞先從分詞詞表裡摘掉（詞庫已併入短語時重跑用）。
    pub refresh: Option<PathBuf>,

    /// 語料。
    pub corpus: Vec<PathBuf>,

    /// 對話語料（`corpus` 裡的一個）：短語在它裡面的次數也要夠 `min_count`。
    pub dialogue: PathBuf,

    /// 次數下限。
    pub min_count: u32,

    /// 短語最多幾個字。
    pub max_chars: usize,
}

/// 次數超過下限這麼多倍的算「夠常見」，形狀規則不再篩它（見 [`rules::passes`]）。
const STRONG_MULTIPLIER: u32 = 5;

/// 一條短語候選。
struct Phrase {
    text: String,
    count: u32,
    pinyin: String,
}

pub fn mine(options: &PhraseOptions, out_dir: &Path) -> Result<(), ConvertError> {
    let mut vocabulary = Vocabulary::load(&options.dict)?;
    let readings = load_readings(&options.dict)?;
    if let Some(path) = &options.refresh {
        let previous = read_words(path)?;
        let before = vocabulary.ids.len();
        vocabulary.ids.retain(|word, _| !previous.contains(word));
        tracing::info!(
            path = %path.display(),
            removed = before - vocabulary.ids.len(),
            "上次的短语已从分词词表摘掉"
        );
    }
    tracing::info!(
        words = vocabulary.ids.len(),
        readings = readings.len(),
        "词表加载完成"
    );
    let known: HashSet<&str> = vocabulary.ids.keys().map(String::as_str).collect();
    let pairs = count(&options.corpus, &options.dialogue, &vocabulary, None)?;
    let frequent_pairs: HashSet<(u32, u32)> = pairs
        .iter()
        .filter(|(_, t)| t.total >= options.min_count)
        .map(|(ids, _)| (ids[0], ids[1]))
        .collect();
    tracing::info!(pairs = frequent_pairs.len(), "够频的二元（三词短语的预筛）");
    let triples = count(
        &options.corpus,
        &options.dialogue,
        &vocabulary,
        Some(&frequent_pairs),
    )?;
    let counts = pairs.into_iter().chain(triples);

    let mut phrases: Vec<Phrase> = Vec::new();
    for (ids, tally) in counts {
        if tally.total < options.min_count || tally.dialogue < options.min_count {
            continue;
        }
        let parts: Vec<&str> = ids
            .iter()
            .map(|id| vocabulary.words[*id as usize].as_str())
            .collect();
        let text = parts.concat();
        let chars = text.chars().count();
        if !(2..=options.max_chars).contains(&chars)
            || known.contains(text.as_str())
            || !rules::passes(&parts, tally.total >= STRONG_MULTIPLIER * options.min_count)
        {
            continue;
        }
        let syllables: Option<Vec<&str>> = parts
            .iter()
            .map(|p| readings.get(*p).map(String::as_str))
            .collect();
        let Some(syllables) = syllables else {
            continue;
        };
        phrases.push(Phrase {
            text,
            count: tally.total,
            pinyin: syllables.join(" "),
        });
    }
    phrases.sort_by(|x, y| y.count.cmp(&x.count).then_with(|| x.text.cmp(&y.text)));
    phrases.dedup_by(|x, y| x.text == y.text);
    let path = out_dir.join("phrases.tsv");
    let mut writer = BufWriter::new(File::create(&path)?);
    writeln!(
        writer,
        "# 由 qingjian-dict-convert phrases 从语料相邻词组合挖出（总次数与对话语料次数都 ≥ {}），读音由成分词拼出。词\\t次数\\t拼音",
        options.min_count
    )?;
    for phrase in &phrases {
        writeln!(
            writer,
            "{}\t{}\t{}",
            phrase.text, phrase.count, phrase.pinyin
        )?;
    }
    writer.flush()?;
    tracing::info!(path = %path.display(), phrases = phrases.len(), "写出完成");
    Ok(())
}

/// 一個詞組合在全部語料與對話語料裡各出現幾次。
#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    /// 全部語料。
    total: u32,

    /// 對話語料。
    dialogue: u32,
}

/// 讀一個 `詞\t…` 檔案的第一列。
fn read_words(path: &Path) -> Result<HashSet<String>, ConvertError> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next().map(str::to_owned))
        .collect())
}

/// 掃語料數相鄰詞組合：`frequent_pairs` 為 `None` 數全部相鄰兩詞，否則數兩段二元都在裡面的相鄰三詞。
fn count(
    corpus: &[PathBuf],
    dialogue: &Path,
    vocabulary: &Vocabulary,
    frequent_pairs: Option<&HashSet<(u32, u32)>>,
) -> Result<HashMap<Vec<u32>, Tally>, ConvertError> {
    let mut counts: HashMap<Vec<u32>, Tally> = HashMap::new();
    let mut tokens = Vec::new();
    let mut lines = 0u64;
    for path in corpus {
        let is_dialogue = path == dialogue;
        tracing::info!(path = %path.display(), dialogue = is_dialogue, "扫描语料");
        let mut bump = |ids: &[u32]| {
            let tally = counts.entry(ids.to_vec()).or_default();
            tally.total += 1;
            if is_dialogue {
                tally.dialogue += 1;
            }
        };
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            lines += 1;
            if lines.is_multiple_of(500_000) {
                tracing::info!(lines, "进度");
            }
            let line: String = line.chars().filter(|c| *c != ' ').collect();
            for run in line.split(|c: char| !is_han(c)) {
                if run.chars().count() < 2 {
                    continue;
                }
                vocabulary.segment(run, &mut tokens);
                // 笑聲串（哈哈哈 切成 哈哈 + 哈）不算相鄰：你哈哈哈 會把 你 + 哈 灌成高頻短語；孤立的 哈（對哈）照常
                let laughter: Vec<bool> = tokens
                    .iter()
                    .map(|t| t.is_some_and(|id| vocabulary.words[id as usize].starts_with('哈')))
                    .collect();
                let in_run = |i: usize| {
                    laughter[i]
                        && ((i > 0 && laughter[i - 1])
                            || (i + 1 < laughter.len() && laughter[i + 1]))
                };
                match frequent_pairs {
                    None => {
                        for (i, window) in tokens.windows(2).enumerate() {
                            if let (Some(a), Some(b)) = (window[0], window[1])
                                && !in_run(i)
                                && !in_run(i + 1)
                            {
                                bump(&[a, b]);
                            }
                        }
                    }
                    Some(pairs) => {
                        for (i, window) in tokens.windows(3).enumerate() {
                            if let (Some(a), Some(b), Some(c)) = (window[0], window[1], window[2])
                                && pairs.contains(&(a, b))
                                && pairs.contains(&(b, c))
                                && !(in_run(i) || in_run(i + 1) || in_run(i + 2))
                            {
                                bump(&[a, b, c]);
                            }
                        }
                    }
                }
            }
        }
        // 記憶體兜底：只出現一次的二元扔掉
        counts.retain(|_, t| t.total > 1);
        tracing::info!(lines, combinations = counts.len(), "本文件扫描完成");
    }
    Ok(counts)
}

/// 詞 → 讀音（同一個詞多個讀音取詞頻最高的那條），讀 `dict` 與同目錄 `dicts/`。
fn load_readings(dict: &Path) -> Result<HashMap<String, String>, ConvertError> {
    let mut best: HashMap<String, (u64, String)> = HashMap::new();
    for path in Vocabulary::files(dict) {
        for line in BufReader::new(File::open(&path)?).lines() {
            let line = line?;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(text), Some(pinyin), Some(frequency)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let frequency: u64 = frequency.trim().parse().unwrap_or(0);
            match best.get_mut(text) {
                Some(entry) if entry.0 >= frequency => {}
                Some(entry) => *entry = (frequency, pinyin.to_owned()),
                None => {
                    best.insert(text.to_owned(), (frequency, pinyin.to_owned()));
                }
            }
        }
    }
    Ok(best.into_iter().map(|(t, (_, p))| (t, p)).collect())
}
