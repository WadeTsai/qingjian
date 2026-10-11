//! 回放評測：把輸入日誌裡每次上屏時的鍵重新餵給引擎，看現在的排序會不會把當時選的詞放在首選。
//!
//! 只在記憶體裡學習（CLI 沒給 `--user-dict` 就是記憶體學習器），不寫任何檔案；按日誌順序回放，
//! 命中的候選照樣上屏，讓上文（個人 n-gram、上一個詞）跟真實使用一樣往前走。
//! 雲端詞、雲端整句、原樣上屏、譯詞不評：它們不是本地排序的結果，只計數。
//! 其他事件（重打、直通、雲端聯想、上文斷開、會話）按 `docs/plan/model-eval.md` 裡的尺子計數。

mod line;
mod report;
mod scheme;
mod tally;

use std::path::Path;

use qingjian_core::{Engine, InputLogEntry, InputSource};
use qingjian_dictionary::CodeTable;

pub use report::Report;

use line::Line;
use scheme::SchemeSwitcher;
use tally::Tally;

/// 跑一遍日誌，返回報告。`code_table` 是給日誌裡形碼那些行用的（`--wubi` 給的碼錶）。
pub fn run(
    engine: &mut Engine,
    path: &Path,
    show_misses: usize,
    code_table: Option<CodeTable>,
) -> Result<Report, ReplayError> {
    let text = std::fs::read_to_string(path).map_err(|source| ReplayError::Read {
        path: path.to_owned(),
        source,
    })?;
    let mut report = Report::default();
    let mut switcher = SchemeSwitcher::new(code_table);
    for (number, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let line: Line = match serde_json::from_str(raw) {
            Ok(line) => line,
            Err(error) => {
                report.unparsable += 1;
                tracing::warn!(line = number + 1, %error, "日志行解析失败，跳过");
                continue;
            }
        };
        match line.entry {
            InputLogEntry::Retract { .. } => report.retracts += 1,
            InputLogEntry::Retype { .. } => report.retypes += 1,
            InputLogEntry::Session { .. } => report.sessions += 1,
            InputLogEntry::Break { .. } => {
                report.breaks += 1;
                engine.break_chain();
            }
            InputLogEntry::Passthrough { text } => {
                // 直通字元也進標點狀態與上文鏈，與真實使用一致
                report.passthrough_chars += text.chars().count();
                for c in text.chars() {
                    engine.note_passthrough(c);
                }
            }
            InputLogEntry::Prediction { .. } => {
                report.predictions += 1;
                report.prediction_pending = true;
            }
            InputLogEntry::Commit(commit) => {
                // 2026-09-12 以前的日誌裡殼每次回車 / 失焦都記一條空的原樣上屏，不算數
                if commit.keys.is_empty() && commit.text.is_empty() {
                    // 那一條對應的是一次回車 / 失焦：上文鏈照樣斷，只是不計數
                    report.empty += 1;
                    engine.clear();
                    engine.break_chain();
                    continue;
                }
                if std::mem::take(&mut report.prediction_pending)
                    && matches!(
                        commit.source,
                        InputSource::Cloud | InputSource::CloudSentence
                    )
                {
                    report.predictions_accepted += 1;
                }
                replay_commit(engine, &mut switcher, &commit, &mut report, show_misses)
            }
        }
    }
    Ok(report)
}

fn replay_commit(
    engine: &mut Engine,
    switcher: &mut SchemeSwitcher,
    commit: &qingjian_core::CommitEntry,
    report: &mut Report,
    show_misses: usize,
) {
    // 這條日誌所屬方案回放不了（形碼但沒給 `--wubi`）：只計數。拿拼音的讀法去喂形碼的鍵會算出
    // 看著像真的、實則無意義的命中率。
    if !switcher.can_replay(&commit.scheme) {
        report.skip(commit.source);
        engine.clear();
        engine.break_chain();
        return;
    }
    switcher.apply(engine, &commit.scheme);
    let Some(tally) = report.tally_for(commit.source) else {
        // 不是本地排序給出的（雲端詞、原樣上屏……）：只計數；那次上屏的詞沒法接進上文，斷鏈。
        // 原樣上屏照樣走一遍 `take_raw`：個人英文詞（`gist`）與「這個串不糾」都是從這裡學的，不走它回放裡的英文候選與糾錯就比真實使用差
        report.skip(commit.source);
        if commit.source == InputSource::Raw && !commit.keys.is_empty() {
            engine.set_english_mode(commit.english);
            feed(engine, &commit.keys);
            engine.take_raw();
        }
        engine.clear();
        engine.break_chain();
        return;
    };
    let tally: &mut Tally = tally;
    tally.total += 1;
    tally.note_logged(commit);
    let scope = if commit.scope.is_empty() {
        commit.keys.as_str()
    } else {
        commit.scope.as_str()
    };
    engine.set_english_mode(commit.english);
    // 日誌裡記的是實際敲鍵：含觸發鍵的按殼那樣逐鍵重喂（進輔碼態、碼段進碼段），
    // 否則按當時的作用域喂。進過輔碼態時留下拼音段，上屏後拿它量學習閉環。
    let pinyin = if commit.keys.contains(engine.aux_code_key()) {
        let (aux, pinyin) = feed(engine, &commit.keys);
        aux.then_some(pinyin)
    } else {
        engine.set_input(scope);
        None
    };
    let query = match engine.query() {
        Ok(query) => query,
        Err(_) => {
            tally.unparsable += 1;
            engine.clear();
            return;
        }
    };
    let position = query
        .candidates
        .items
        .iter()
        .position(|c| c.text == commit.text);
    match position {
        Some(0) => tally.top1 += 1,
        Some(index) if index < 5 => tally.top5 += 1,
        Some(_) => tally.found_later += 1,
        None => tally.missing += 1,
    }
    if let Some(index) = position {
        tally.rank_sum += index + 1;
    }
    if commit.corrected {
        tally.corrected_then += 1;
        if query.correction.is_some() {
            tally.corrected_now += 1;
        }
    }
    if position != Some(0) && report.misses.len() < show_misses {
        let top: Vec<String> = query
            .candidates
            .items
            .iter()
            .take(3)
            .map(|c| c.text.clone())
            .collect();
        report.misses.push(format!(
            "[{}] {scope:<16} 选了 {:<8} 现在前三 {}{}",
            source_label(commit.source),
            commit.text,
            top.join(" / "),
            position.map_or(String::from("（不在候选里）"), |i| format!(
                "（第 {} 位）",
                i + 1
            )),
        ));
    }
    // 照著當時的選擇上屏，讓上下文往前走；不在候選裡就原樣清掉
    match position.map(|i| query.candidates.items[i].clone()) {
        Some(candidate) => {
            engine.commit(&candidate);
            engine.clear();
        }
        None => engine.clear(),
    }
    // 輔碼選詞的閉環：學習記完之後，同樣的拼音改成純拼音輸入，這個詞該排到首選
    if let Some(pinyin) = pinyin {
        report.aux_total += 1;
        engine.set_input(&pinyin);
        let top1 = engine
            .query()
            .ok()
            .and_then(|query| query.candidates.items.first().map(|c| c.text.clone()));
        if top1.as_deref() == Some(commit.text.as_str()) {
            report.aux_top1 += 1;
        }
        engine.clear();
    }
}

/// 與殼一樣逐個喂鍵：觸發鍵進輔碼態、之後的字母進碼段，其餘進拼音緩衝區。
/// 返回 (是否進過輔碼態, 進輔碼態那一刻的拼音緩衝區)。
fn feed(engine: &mut Engine, keys: &str) -> (bool, String) {
    engine.clear();
    let mut aux = false;
    let mut pinyin = String::new();
    for key in keys.chars() {
        if engine.aux_trigger(key) {
            engine.enter_aux();
            aux = true;
            pinyin = engine.composition().text().to_owned();
        } else if engine.in_aux() {
            if !engine.push_aux_code(key) {
                engine.clear_aux();
                engine.push(key);
            }
        } else {
            engine.push(key);
        }
    }
    (aux, pinyin)
}

/// 回放的錯誤。
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("cannot read input log {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// 沒命中例子裡標來源用的短名，與報告各計數板同名。
fn source_label(source: InputSource) -> &'static str {
    match source {
        InputSource::Word => "词",
        InputSource::Sentence => "整句",
        InputSource::English => "英文",
        _ => "其他",
    }
}
