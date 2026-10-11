//! Unicode CLDR emoji annotations（中文或英文）→ `詞\temoji …`。
//!
//! JSON 結構：`{"annotations" | "annotationsDerived": {"annotations": {"😀": {"default": [關鍵詞…], "tts": [名字]}}}}`。
//! 名字正好是這個詞的排在前面，其餘按「名字裡含這個詞」再按關鍵詞位置排（中文關鍵詞大致按相關度列，英文是字母序，所以名字命中更重要）；
//! 單字詞（愛、小）只收名字裡含這個字的 emoji，否則 愛 會配出 👨‍🍼 這種只沾邊的。
//! 跳過膚色修飾符、單獨的 ASCII 字元、幾何形狀和帶膚色的變體（表會大三倍，候選裡也用不上）。

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::ConvertError;

/// 關鍵詞命中：(emoji, 這個詞在它關鍵詞表裡的位置, 它的名字裡含這個詞, 含這個詞的最短名字長度)。
/// 名字越短越基本（red heart 比 heart with arrow 更該排前）。
type KeywordHit = (String, usize, bool, usize);

pub fn convert(inputs: &[PathBuf], output: &Path, language: &str) -> Result<(), ConvertError> {
    // 詞 → (名字命中的 emoji, 關鍵詞命中)
    let mut table: BTreeMap<String, (Vec<String>, Vec<KeywordHit>)> = BTreeMap::new();
    let mut emojis = 0;
    for path in inputs {
        let json: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        let root = json
            .get("annotations")
            .or_else(|| json.get("annotationsDerived"))
            .and_then(|v| v.get("annotations"))
            .and_then(Value::as_object)
            .ok_or_else(|| ConvertError::Format {
                path: path.clone(),
                line: 0,
                reason: "expected annotations.annotations object".to_owned(),
            })?;
        for (emoji, entry) in root {
            if !keep(emoji) {
                continue;
            }
            emojis += 1;
            let words = |key: &str| -> Vec<String> {
                entry
                    .get(key)
                    .and_then(Value::as_array)
                    .map(|list| {
                        list.iter()
                            .filter_map(Value::as_str)
                            .map(str::trim)
                            .filter(|w| !w.is_empty() && !w.contains(' '))
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let names = words("tts");
            for name in &names {
                table.entry(name.clone()).or_default().0.push(emoji.clone());
            }
            // 「名字裡含這個詞」按完整名字判斷：英文名字多是短語（grinning face），不能只看單詞名
            let full_names: Vec<String> = entry
                .get("tts")
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(Value::as_str)
                        .map(|s| s.trim().to_lowercase())
                        .collect()
                })
                .unwrap_or_default();
            for (position, keyword) in words("default").into_iter().enumerate() {
                let keyword = keyword.to_lowercase();
                // 英文關鍵詞與名字裡的變形要對得上（smile / smiling）：去掉詞尾 e 再找
                let stem = keyword.strip_suffix('e').unwrap_or(&keyword);
                let name_len = full_names
                    .iter()
                    .filter(|n| {
                        n.contains(keyword.as_str()) || (stem.len() >= 3 && n.contains(stem))
                    })
                    .map(|n| n.chars().count())
                    .min();
                table.entry(keyword).or_default().1.push((
                    emoji.clone(),
                    position,
                    name_len.is_some(),
                    name_len.unwrap_or(usize::MAX),
                ));
            }
        }
        tracing::info!(path = %path.display(), "已读取");
    }
    let mut file = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        file,
        "# 由 qingjian-dict-convert 从 Unicode CLDR {language} annotations 生成（Unicode License v3）。词\\temoji emoji …"
    )?;
    let mut words = 0;
    for (word, (named, mut keyed)) in table {
        let single = word.chars().count() == 1;
        // 單字詞只要名字裡含這個字的；名字裡含詞的靠前（名字越短越基本），再按關鍵詞位置，老符號（U+1F000 以下）最後
        keyed.retain(|(_, _, in_name, _)| !single || *in_name);
        keyed.sort_by_key(|(emoji, position, in_name, name_len)| {
            (!*in_name, *name_len, *position, legacy(emoji))
        });
        let mut list: Vec<&str> = Vec::new();
        for emoji in named.iter().chain(keyed.iter().map(|(e, _, _, _)| e)) {
            if !list.contains(&emoji.as_str()) {
                list.push(emoji);
            }
        }
        if list.is_empty() {
            continue;
        }
        words += 1;
        writeln!(file, "{word}\t{}", list.join(" "))?;
    }
    file.flush()?;
    tracing::info!(path = %output.display(), emojis, words, "写出完成");
    Ok(())
}

/// 要不要收這個 emoji：跳過純 ASCII、膚色修飾符本身、帶膚色修飾符的變體、幾何形狀（▪ ▫ ◾ 這類當候選只是噪音）。
fn keep(emoji: &str) -> bool {
    if emoji.is_ascii() {
        return false;
    }
    !emoji
        .chars()
        .any(|c| ('\u{1F3FB}'..='\u{1F3FF}').contains(&c) || ('\u{25A0}'..='\u{25FF}').contains(&c))
}

/// 不在 emoji 專屬區（U+1F000 以上）的老符號。
fn legacy(emoji: &str) -> bool {
    emoji.chars().next().is_none_or(|c| (c as u32) < 0x1F000)
}
