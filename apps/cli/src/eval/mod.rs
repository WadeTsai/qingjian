//! 整句評測：拿使用者自己寫的中文文本，轉成他會敲的全拼，冷啟動餵給引擎，看整句轉換能不能把原句還原出來。
//!
//! 與 `--replay` 不同，這把尺子不依賴輸入日誌裡「當時選了什麼」（那多半是當時的引擎自己的輸出），
//! 只看原文；整句排序、語言模型的改動先在同一份句子集上比過再合。
//! 輸入既可以是原始文本（一行一段，按標點切句、漢字轉拼音），也可以是之前 `--eval-save` 凍結下來的
//! `句子\t拼音\t上文` 三列檔案；後者保證不同時間、不同分支比的是同一份句子。
//! 每句獨立：不上屏、不學習，只把這句在原文裡的上文寫進輸入歷史給整句轉換用。

pub mod coverage;
mod extract;
pub mod generate;
pub mod p2c;
mod pair;
mod report;
mod transcribe;

use std::collections::HashSet;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use qingjian_core::Engine;

pub use report::Report;

use pair::Pair;
use transcribe::Transcriber;

/// 跑一遍評測集，返回報告；`save` 給了就把用到的句子集寫成三列檔案。
pub fn run(
    engine: &mut Engine,
    paths: &[PathBuf],
    save: Option<&Path>,
    show_misses: usize,
    details: Option<&Path>,
) -> Result<Report, EvalError> {
    // 碼錶覆蓋率與句子集無關，裝了碼錶就先算（詞庫按詞頻兩段口徑）
    let mut report = Report {
        coverage: coverage::measure(engine),
        ..Report::default()
    };
    let pairs = collect(engine, paths, &mut report)?;
    if let Some(path) = save {
        let mut text = String::new();
        for pair in &pairs {
            text.push_str(&pair.to_line());
            text.push('\n');
        }
        std::fs::write(path, text).map_err(|source| EvalError::Write {
            path: path.to_owned(),
            source,
        })?;
        tracing::info!(path = %path.display(), count = pairs.len(), "句子集已保存");
    }
    let mut writer = details
        .map(|path| {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map(BufWriter::new)
                .map_err(|source| EvalError::Write {
                    path: path.to_owned(),
                    source,
                })
        })
        .transpose()?;
    for (index, pair) in pairs.iter().enumerate() {
        let row = evaluate(engine, pair, &mut report, show_misses);
        if let Some(writer) = &mut writer {
            writeln!(writer, "{row}").map_err(|source| EvalError::Write {
                path: details.expect("writer has path").to_owned(),
                source,
            })?;
        }
        if (index + 1) % 500 == 0 {
            tracing::info!(done = index + 1, total = pairs.len(), "整句评测进度");
        }
    }
    if let Some(writer) = &mut writer {
        writer.flush().map_err(|source| EvalError::Write {
            path: details.expect("writer has path").to_owned(),
            source,
        })?;
    }
    Ok(report)
}

/// 讀全部檔案，得到去重後的句子集：有製表符的檔案按凍結格式讀，其餘當原始文本抽句、轉拼音。
pub(super) fn collect(
    engine: &Engine,
    paths: &[PathBuf],
    report: &mut Report,
) -> Result<Vec<Pair>, EvalError> {
    let mut transcriber: Option<Transcriber> = None;
    let mut seen: HashSet<String> = HashSet::new();
    let mut pairs = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(path).map_err(|source| EvalError::Read {
            path: path.clone(),
            source,
        })?;
        if text.contains('\t') {
            for line in text.lines() {
                if let Some(pair) = Pair::parse(line)
                    && seen.insert(pair.text.clone())
                {
                    pairs.push(pair);
                }
            }
            continue;
        }
        let transcriber = transcriber.get_or_insert_with(|| {
            let dictionaries =
                std::iter::once(engine.dictionary()).chain(engine.extra_dictionaries());
            let transcriber = Transcriber::new(dictionaries);
            tracing::info!(words = transcriber.len(), "读音反查表已建");
            transcriber
        });
        for extracted in extract::extract(&text) {
            if !seen.insert(extracted.text.clone()) {
                continue;
            }
            report.extracted += 1;
            match transcriber.transcribe(&extracted.text, engine.language_model()) {
                Some(pinyin) => pairs.push(Pair {
                    text: extracted.text,
                    pinyin,
                    context: extracted.context,
                }),
                None => report.untranscribable += 1,
            }
        }
    }
    Ok(pairs)
}

/// 評一句：清空引擎狀態、寫入上文、喂拼音、看候選。
fn evaluate(
    engine: &mut Engine,
    pair: &Pair,
    report: &mut Report,
    show_misses: usize,
) -> serde_json::Value {
    report.total += 1;
    engine.clear();
    engine.break_chain();
    engine.history_mut().clear();
    engine.history_mut().record(&pair.context);
    engine.set_input(&pair.pinyin);
    let started = Instant::now();
    let query = match engine.query() {
        Ok(query) => query,
        Err(error) => {
            report.unparsable += 1;
            engine.clear();
            return serde_json::json!({"text": pair.text, "pinyin": pair.pinyin, "error": error.to_string()});
        }
    };
    // 非同步重打分：像殼一樣停頓後請求、等結果、再查一次；等的時間也算進查詢耗時
    let query = if crate::rescoring::settle(engine) {
        engine.query().unwrap_or(query)
    } else {
        query
    };
    let elapsed = started.elapsed();
    report.query_time += elapsed;
    report.slowest_query = report.slowest_query.max(elapsed);
    let items = &query.candidates.items;
    let position = items.iter().position(|c| c.text == pair.text);
    if position == Some(0) {
        report.top1 += 1;
    }
    if let Some(rank) = position {
        report.top3 += usize::from(rank < 3);
        report.top5 += usize::from(rank < 5);
    }
    // 第一個蓋住全部拼音的候選就是整句轉換的答案（整句本身是個詞時也可能是詞庫詞）
    let length = pair.text.chars().count();
    let sentence = items.iter().find(|c| c.text.chars().count() == length);
    report.chars_total += length;
    if let Some(sentence) = sentence {
        if sentence.text == pair.text {
            report.sentence_hit += 1;
        }
        report.chars_correct += sentence
            .text
            .chars()
            .zip(pair.text.chars())
            .filter(|(a, b)| a == b)
            .count();
    }
    if position != Some(0) && report.misses.len() < show_misses {
        let top: Vec<&str> = items.iter().take(3).map(|c| c.text.as_str()).collect();
        report.misses.push(format!(
            "{:<20} {:<28} 现在前三 {}{}",
            pair.text,
            pair.pinyin,
            top.join(" / "),
            position.map_or(String::from("（不在候选里）"), |i| format!(
                "（第 {} 位）",
                i + 1
            )),
        ));
    }
    let row = serde_json::json!({
        "text": pair.text, "pinyin": pair.pinyin, "context": pair.context,
        "top": items.first().map(|c| c.text.as_str()),
        "sentence": sentence.map(|c| c.text.as_str()),
        "position": position,
        "query_ms": elapsed.as_secs_f64() * 1000.0,
        "candidates": items.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
    });
    engine.clear();
    row
}

/// 整句評測的錯誤。
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("cannot read evaluation text {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot write sentence set {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("P2C generation failed: {0}")]
    Generate(#[from] qingjian_neural::NeuralError),
}
