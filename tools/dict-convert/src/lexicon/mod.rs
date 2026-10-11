//! 青簡自己的詞庫：從「輸入法字詞庫_分類整理版」資料包（規範字 + 現代漢語常用詞 + THUOCL 領域詞）建 `dict.tsv`。
//!
//! 通用詞自帶拼音，直接規範化；規範字與領域詞沒有拼音，讀音來自 Unihan（Unicode 許可）：
//! 單字按 kHanyuPinlu / kXHC1983 / kMandarin 給全部讀音與權重，多字詞按字拼接，多音字先看 LLM 標註
//! （`gloss-gen pinyin` 的 JSONL，逐字對照 Unihan 校驗）、再看通用詞裡該字最常見的讀音、再看 kHanyuPinlu。
//! 詞頻來自自己的語料統計（`bigram` 子命令的 lm-unigram.tsv），沒統計到的按詞表排序號 / 文件頻次給一個很小的底值。
//!
//! 兩遍跑：第一遍沒有詞頻，只為分詞與 `--emit-ambiguous` 出待標註詞表；標註、統計完再跑一遍寫最終 dict.tsv。
//!
//! 領域詞拆開出：語料裡出現夠多的（`domain_keep_min`）是通用詞，留在基礎詞庫 `dict.tsv`；其餘按來原始檔各寫一本
//! `dicts/<領域>.tsv` 與帶後設資料的 `dicts/<領域>.qj`，隨包分發、預設關閉，使用者按需開啟。基礎詞庫與各領域詞庫互不重疊。

mod annotations;
mod corpus;
mod entry;
pub(crate) mod pack;
mod readings;
mod tone;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::Path;

use qingjian_core::parser::is_syllable;
use qingjian_dictionary::Dictionary;
use qingjian_format::Metadata;

use crate::error::ConvertError;
use entry::LexiconEntry;
use pack::Pack;
use readings::CharReadings;

/// 領域詞最長几個字：再長是法規名、詩句，輸入法用不上。
const MAX_WORD_CHARS: usize = 10;

/// 沒統計到詞頻時通用詞的底值區間上限（按排序號線性遞減到 1）。
const COMMON_FLOOR: f64 = 100.0;

/// 沒統計到詞頻時領域詞的底值上限（按文件頻次取對數）。
const DOMAIN_FLOOR: u32 = 30;

/// 沒統計到詞頻時單字按字表級別給的底值。
const CHAR_FLOOR: [u32; 3] = [30, 10, 3];

/// 單字不在 kHanyuPinlu 裡的次要讀音佔主讀音權重的比例。
const MINOR_READING_SHARE: f64 = 0.05;

/// 常用詞表自帶的讀音與 LLM 標註不一致時，原表讀音降到標註讀音詞頻的幾分之一保留（怎麼打都找得到，但不搶首選）。
const DISPUTED_READING_DIVISOR: u32 = 8;

/// 領域詞庫的中文名（檔名主幹 → 名稱），寫進 `.qj` 後設資料，偏好設定「詞庫」頁顯示它。
const DOMAIN_NAMES: [(&str, &str); 11] = [
    ("animals", "动物"),
    ("automotive", "汽车"),
    ("finance", "财经"),
    ("food", "饮食"),
    ("historical_figures", "历史人物"),
    ("idioms", "成语"),
    ("it_computing", "IT 与计算机"),
    ("law", "法律"),
    ("medicine", "医学"),
    ("places", "地名"),
    ("poetry_lines", "诗词名句"),
];

/// 領域詞庫的許可證與署名（THUOCL）。
const DOMAIN_LICENSE: &str = "MIT AND Unicode-3.0";
const DOMAIN_ATTRIBUTION: &str =
    "THUOCL（清华大学自然语言处理实验室，MIT）；读音 Unihan（Unicode）";
const DOMAIN_SOURCE: &str = "https://github.com/thunlp/THUOCL";

/// 建詞庫。`pinyin` 是 LLM 標註 JSONL，`frequency` 是 lm-unigram.tsv，`emit_ambiguous` 寫出仍靠猜的多音字詞，
/// `domain_keep_min` 是領域詞留在基礎詞庫的最低語料次數（沒有詞頻時按領域詞處理，全部拆出去）。
#[allow(clippy::too_many_arguments)]
pub fn convert(
    pack_dir: &Path,
    unihan: &Path,
    pinyin: Option<&Path>,
    frequency: Option<&Path>,
    emit_ambiguous: Option<&Path>,
    extra_words: &[std::path::PathBuf],
    domain_keep_min: u64,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let mut pack = Pack::load(pack_dir)?;
    // 語料挖出來的詞當作一類領域詞併入：次數當文件頻次（也就是沒有語料詞頻時的底值來源）
    for path in extra_words {
        let before = pack.domain.len();
        for word in corpus::extra_words(path)? {
            pack.domain.push(pack::DomainRow {
                text: word.text,
                df: word.count,
                domain: None,
                syllables: word.syllables,
            });
        }
        tracing::info!(path = %path.display(), rows = pack.domain.len() - before, "额外词已读取");
    }
    let readings = CharReadings::load(unihan)?;
    let annotations = match pinyin {
        Some(path) => annotations::load(path)?,
        None => HashMap::new(),
    };
    let mut counts = match frequency {
        Some(path) => corpus::load(path)?,
        None => HashMap::new(),
    };
    // 挖出來的詞在語料裡的次數就是它的詞頻（分詞時它被拆成單字，一元表裡沒有）
    for path in extra_words {
        for word in corpus::extra_words(path)? {
            counts.entry(word.text).or_insert(word.count);
        }
    }
    tracing::info!(
        chars = pack.chars.len(),
        common = pack.common.len(),
        domain = pack.domain.len(),
        readings = readings.len(),
        annotations = annotations.len(),
        counts = counts.len(),
        "数据加载完成"
    );

    // 通用詞裡每個字各種讀音出現的次數：多音字在詞裡猜讀音的第一依據
    let mut common_stats: HashMap<char, HashMap<String, u32>> = HashMap::new();
    for row in &pack.common {
        for (ch, syllable) in row.text.chars().zip(&row.syllables) {
            *common_stats
                .entry(ch)
                .or_default()
                .entry(syllable.clone())
                .or_default() += 1;
        }
    }

    let mut entries: BTreeMap<(String, Vec<String>), LexiconEntry> = BTreeMap::new();
    // 拆出去的領域詞：領域 → 詞條
    let mut domains: BTreeMap<String, BTreeMap<(String, Vec<String>), LexiconEntry>> =
        BTreeMap::new();
    let mut insert = |entry: LexiconEntry| {
        let key = (entry.text.clone(), entry.syllables.clone());
        entries
            .entry(key)
            .and_modify(|e| e.frequency = e.frequency.max(entry.frequency))
            .or_insert(entry);
    };
    let mut dropped = 0usize;

    // 單字：規範字表全部 + 通用詞裡用到而字表沒有的
    let mut chars: Vec<(char, Option<u8>)> =
        pack.chars.iter().map(|c| (c.ch, Some(c.level))).collect();
    let listed: HashSet<char> = chars.iter().map(|c| c.0).collect();
    let mut extra: Vec<char> = pack
        .common
        .iter()
        .flat_map(|row| row.text.chars())
        .filter(|c| !listed.contains(c))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    extra.sort_unstable();
    chars.extend(extra.into_iter().map(|c| (c, None)));
    for (ch, level) in &chars {
        let weighted = readings.weighted(*ch, MINOR_READING_SHARE);
        let mut valid: Vec<(String, f64)> = weighted
            .iter()
            .filter(|(syllable, _)| is_syllable(syllable))
            .cloned()
            .collect();
        // 嗯 在 Unihan 裡只有 ng / n，不是拼音音節會整個字丟掉；各家輸入法都用 en 打它
        if valid.is_empty() && weighted.iter().any(|(s, _)| s == "n" || s == "ng") {
            valid.push(("en".to_owned(), 1.0));
        }
        if valid.is_empty() {
            dropped += 1;
            continue;
        }
        let total = counts.get(&ch.to_string()).copied().unwrap_or_else(|| {
            u64::from(CHAR_FLOOR[usize::from(level.unwrap_or(3).saturating_sub(1).min(2))])
        });
        let sum: f64 = valid.iter().map(|(_, w)| w).sum();
        for (syllable, weight) in valid {
            let frequency = ((total as f64) * weight / sum).round().max(1.0) as u32;
            insert(LexiconEntry {
                text: ch.to_string(),
                syllables: vec![syllable],
                frequency,
            });
        }
    }

    // 通用詞：自帶拼音，但原表對多音字有錯（重慶 zhong qing、長大 chang da），含多音字的詞以 LLM 標註為主讀音，
    // 原表讀音不同時降權保留
    let common_total = pack.common.len().max(1) as f64;
    let char_texts: HashSet<String> = chars.iter().map(|(ch, _)| ch.to_string()).collect();
    let mut common_texts: HashSet<&str> = HashSet::new();
    let mut ambiguous: BTreeMap<String, ()> = BTreeMap::new();
    let mut disputed = 0usize;
    let polyphonic = |text: &str| {
        text.chars().any(|ch| {
            readings
                .all(ch)
                .into_iter()
                .filter(|s| is_syllable(s))
                .count()
                > 1
        })
    };
    for row in &pack.common {
        if !row.syllables.iter().all(|s| is_syllable(s)) {
            dropped += 1;
            continue;
        }
        // 單字以字表那條為準：那裡按讀音訊次分攤了詞頻；通用詞表裡 了 有 le / liao 兩條，
        // 都給整個字的詞頻會讓 liao 下的 了 壓過 聊 / 料
        if char_texts.contains(&row.text) {
            common_texts.insert(&row.text);
            continue;
        }
        common_texts.insert(&row.text);
        let frequency = counts.get(&row.text).map_or_else(
            || (1.0 + COMMON_FLOOR * (1.0 - row.rank as f64 / common_total)).round() as u64,
            |&c| c.max(1),
        );
        let frequency = frequency.min(u64::from(u32::MAX)) as u32;
        let verified = annotations
            .get(&row.text)
            .filter(|a| readings.accepts_word(&row.text, a) && a.iter().all(|s| is_syllable(s)));
        match verified {
            Some(annotation) if *annotation != row.syllables => {
                disputed += 1;
                insert(LexiconEntry {
                    text: row.text.clone(),
                    syllables: annotation.clone(),
                    frequency,
                });
                insert(LexiconEntry {
                    text: row.text.clone(),
                    syllables: row.syllables.clone(),
                    frequency: (frequency / DISPUTED_READING_DIVISOR).max(1),
                });
            }
            Some(_) => insert(LexiconEntry {
                text: row.text.clone(),
                syllables: row.syllables.clone(),
                frequency,
            }),
            None => {
                if polyphonic(&row.text) {
                    ambiguous.insert(row.text.clone(), ());
                }
                insert(LexiconEntry {
                    text: row.text.clone(),
                    syllables: row.syllables.clone(),
                    frequency,
                });
            }
        }
    }

    // 領域詞：沒有拼音，按字推；多音字先看標註。額外詞檔案給了讀音的（短語層）直接用
    let mut guessed = 0usize;
    let mut given = 0usize;
    let mut annotated = 0usize;
    let mut rejected_annotations = 0usize;
    let mut seen_domain: HashSet<&str> = HashSet::new();
    for row in &pack.domain {
        let text = row.text.as_str();
        let chars_count = text.chars().count();
        if !(2..=MAX_WORD_CHARS).contains(&chars_count)
            || common_texts.contains(text)
            || !seen_domain.insert(text)
        {
            continue;
        }
        // 額外詞檔案給了讀音的（短語層由成分詞拼出）直接用；否則先看 LLM 標註，再按字推
        let given_syllables = row
            .syllables
            .as_ref()
            .filter(|s| readings.accepts_word(text, s) && s.iter().all(|s| is_syllable(s)));
        let syllables = match (given_syllables, annotations.get(text)) {
            (Some(candidate), _) => {
                given += 1;
                candidate.clone()
            }
            (None, Some(candidate))
                if readings.accepts_word(text, candidate)
                    && candidate.iter().all(|s| is_syllable(s)) =>
            {
                annotated += 1;
                candidate.clone()
            }
            (None, other) => {
                if other.is_some() {
                    rejected_annotations += 1;
                }
                let mut guess = Vec::with_capacity(chars_count);
                let mut polyphonic = false;
                let mut ok = true;
                for ch in text.chars() {
                    let options: Vec<String> = readings
                        .all(ch)
                        .into_iter()
                        .filter(|s| is_syllable(s))
                        .collect();
                    if options.is_empty() {
                        ok = false;
                        break;
                    }
                    if options.len() > 1 {
                        polyphonic = true;
                    }
                    let best = common_stats
                        .get(&ch)
                        .and_then(|stats| {
                            stats
                                .iter()
                                .filter(|(s, _)| options.contains(s))
                                .max_by_key(|(_, n)| **n)
                                .map(|(s, _)| s.clone())
                        })
                        .unwrap_or_else(|| options[0].clone());
                    guess.push(best);
                }
                if !ok {
                    dropped += 1;
                    continue;
                }
                if polyphonic {
                    ambiguous.insert(text.to_owned(), ());
                }
                guessed += 1;
                guess
            }
        };
        let corpus_count = counts.get(text).copied();
        let frequency = corpus_count.map_or_else(
            || 1 + ((row.df as f64 + 1.0).log2().round() as u32).min(DOMAIN_FLOOR),
            |c| c.max(1).min(u64::from(u32::MAX)) as u32,
        );
        let entry = LexiconEntry {
            text: text.to_owned(),
            syllables,
            frequency,
        };
        // 語料裡常見的領域詞其實是通用詞（醫療器械、侵權行為），留在基礎詞庫；其餘進各自的領域詞庫
        match &row.domain {
            Some(domain) if corpus_count.unwrap_or(0) < domain_keep_min => {
                domains
                    .entry(domain.clone())
                    .or_default()
                    .insert((entry.text.clone(), entry.syllables.clone()), entry);
            }
            _ => insert(entry),
        }
    }

    std::fs::create_dir_all(out_dir)?;
    let output = out_dir.join("dict.tsv");
    let mut file = BufWriter::new(std::fs::File::create(&output)?);
    writeln!(
        file,
        "# 青简基础词库，由 qingjian-dict-convert lexicon 生成。词\t音节\t词频\n\
# 来源：通用规范汉字表（8105 字）；现代汉语常用词表（liuxilu 校对版）；THUOCL 领域词（MIT，清华大学自然语言处理实验室）；\n\
# 读音：Unihan（Unicode License）+ LLM 标注多音字词；词频：青简自己的语料统计（中文维基 CC BY-SA 4.0、LCCC MIT）。"
    )?;
    for entry in entries.values() {
        writeln!(
            file,
            "{}\t{}\t{}",
            entry.text,
            entry.syllables.join(" "),
            entry.frequency
        )?;
    }
    file.flush()?;
    write_domains(&domains, out_dir)?;
    if let Some(path) = emit_ambiguous {
        let mut file = BufWriter::new(std::fs::File::create(path)?);
        for word in ambiguous.keys() {
            writeln!(file, "{word}")?;
        }
        file.flush()?;
        tracing::info!(path = %path.display(), words = ambiguous.len(), "待标注的多音字词已写出");
    }
    tracing::info!(
        path = %output.display(),
        entries = entries.len(),
        domain_libraries = domains.len(),
        domain_entries = domains.values().map(BTreeMap::len).sum::<usize>(),
        given,
        annotated,
        guessed,
        disputed_common_readings = disputed,
        still_ambiguous = ambiguous.len(),
        rejected_annotations,
        dropped,
        "词库写出完成"
    );
    Ok(())
}

/// 每個領域寫一本 `dicts/<領域>.tsv`（與主詞庫同格式）和一本帶後設資料的 `dicts/<領域>.qj`。
fn write_domains(
    domains: &BTreeMap<String, BTreeMap<(String, Vec<String>), LexiconEntry>>,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let dir = out_dir.join("dicts");
    std::fs::create_dir_all(&dir)?;
    for (stem, entries) in domains {
        let name = DOMAIN_NAMES
            .iter()
            .find(|(key, _)| key == stem)
            .map_or(stem.as_str(), |(_, name)| name);
        let mut tsv = format!(
            "# 青简领域词库：{name}，由 qingjian-dict-convert lexicon 从 THUOCL 拆出，基础词库里没有的部分。词\t音节\t词频\n"
        );
        for entry in entries.values() {
            tsv.push_str(&entry.text);
            tsv.push('\t');
            tsv.push_str(&entry.syllables.join(" "));
            tsv.push('\t');
            tsv.push_str(&entry.frequency.to_string());
            tsv.push('\n');
        }
        let tsv_path = dir.join(format!("{stem}.tsv"));
        std::fs::write(&tsv_path, &tsv)?;
        let dictionary = Dictionary::parse(&tsv)?;
        let metadata = Metadata {
            name: format!("青简领域词库：{name}"),
            license: DOMAIN_LICENSE.to_owned(),
            attribution: DOMAIN_ATTRIBUTION.to_owned(),
            source: DOMAIN_SOURCE.to_owned(),
            generator: format!("qingjian-dict-convert {}", env!("CARGO_PKG_VERSION")),
            ..Metadata::default()
        };
        let qj_path = dir.join(format!("{stem}.qj"));
        dictionary.write_qj(&qj_path, &metadata)?;
        tracing::info!(domain = %name, entries = entries.len(), path = %qj_path.display(), "领域词库已写出");
    }
    Ok(())
}
